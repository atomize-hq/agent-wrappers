use std::time::Duration;

use claude_code::{ClaudeStreamJsonEvent, ClaudeStreamJsonParseError, ClaudeStreamJsonParser};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader},
    sync::mpsc,
};

const SYSTEM_INIT: &str = include_str!("fixtures/stream_json/v1/system_init.jsonl");
const USER_MESSAGE: &str = include_str!("fixtures/stream_json/v1/user_message.jsonl");

fn first_nonempty_line(text: &str) -> &str {
    text.lines()
        .find(|line| !line.chars().all(|ch| ch.is_whitespace()))
        .expect("fixture contains a non-empty line")
}

async fn stream_parse_typed_events<R>(
    reader: R,
    tx: mpsc::Sender<Result<ClaudeStreamJsonEvent, ClaudeStreamJsonParseError>>,
) where
    R: AsyncRead + Unpin + Send + 'static,
{
    let mut parser = ClaudeStreamJsonParser::new();
    let mut lines = BufReader::new(reader).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        match parser.parse_line(&line) {
            Ok(None) => {}
            Ok(Some(ev)) => {
                if tx.send(Ok(ev)).await.is_err() {
                    break;
                }
            }
            Err(err) => {
                if tx.send(Err(err)).await.is_err() {
                    break;
                }
            }
        }
    }
}

#[tokio::test]
async fn yields_events_incrementally_before_eof() {
    let (mut writer, reader) = tokio::io::duplex(1024);
    let (tx, mut rx) =
        mpsc::channel::<Result<ClaudeStreamJsonEvent, ClaudeStreamJsonParseError>>(8);

    let handle = tokio::spawn(stream_parse_typed_events(reader, tx));

    let init = first_nonempty_line(SYSTEM_INIT);
    writer.write_all(init.as_bytes()).await.unwrap();
    writer.write_all(b"\n").await.unwrap();

    let first = tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("expected first event before writer closes")
        .expect("channel open")
        .expect("event parses");
    assert!(matches!(first, ClaudeStreamJsonEvent::SystemInit { .. }));
    assert!(
        !handle.is_finished(),
        "stream task should remain active before EOF"
    );

    let user = first_nonempty_line(USER_MESSAGE);
    writer.write_all(user.as_bytes()).await.unwrap();
    writer.write_all(b"\n").await.unwrap();
    drop(writer);

    let second = tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("expected second event")
        .expect("channel open")
        .expect("event parses");
    assert!(matches!(second, ClaudeStreamJsonEvent::UserMessage { .. }));

    tokio::time::timeout(Duration::from_secs(1), handle)
        .await
        .expect("stream task should finish after EOF")
        .unwrap();
}

#[tokio::test]
async fn crlf_and_blank_lines_are_ignored_in_streaming_mode() {
    let (mut writer, reader) = tokio::io::duplex(1024);
    let (tx, mut rx) =
        mpsc::channel::<Result<ClaudeStreamJsonEvent, ClaudeStreamJsonParseError>>(8);

    let handle = tokio::spawn(stream_parse_typed_events(reader, tx));

    writer.write_all(b"\r\n").await.unwrap();
    writer.write_all(b"   \r\n").await.unwrap();

    let init = first_nonempty_line(SYSTEM_INIT);
    writer
        .write_all(format!("{init}\r\n").as_bytes())
        .await
        .unwrap();

    writer.write_all(b"\r\n").await.unwrap();

    let user = first_nonempty_line(USER_MESSAGE);
    writer
        .write_all(format!("{user}\r\n").as_bytes())
        .await
        .unwrap();

    drop(writer);

    let a = tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let b = tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(matches!(a, ClaudeStreamJsonEvent::SystemInit { .. }));
    assert!(matches!(b, ClaudeStreamJsonEvent::UserMessage { .. }));

    tokio::time::timeout(Duration::from_secs(1), handle)
        .await
        .expect("stream task should finish after EOF")
        .unwrap();

    let extra = tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("receiver should resolve after channel close");
    assert!(extra.is_none(), "no extra events expected");
}

#[tokio::test]
async fn parse_errors_are_redacted_and_do_not_embed_raw_line_content() {
    let (mut writer, reader) = tokio::io::duplex(1024);
    let (tx, mut rx) =
        mpsc::channel::<Result<ClaudeStreamJsonEvent, ClaudeStreamJsonParseError>>(8);

    let handle = tokio::spawn(stream_parse_typed_events(reader, tx));

    let secret = "VERY_SECRET_SHOULD_NOT_APPEAR";
    let bad_line = format!("not json {secret}\n");
    writer.write_all(bad_line.as_bytes()).await.unwrap();

    let init = first_nonempty_line(SYSTEM_INIT);
    writer.write_all(init.as_bytes()).await.unwrap();
    writer.write_all(b"\n").await.unwrap();
    drop(writer);

    let first = tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .unwrap()
        .unwrap()
        .expect_err("expected parse error first");
    assert!(!first.message.contains(secret));
    assert!(!first.details.contains(secret));

    let second = tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(matches!(second, ClaudeStreamJsonEvent::SystemInit { .. }));

    tokio::time::timeout(Duration::from_secs(1), handle)
        .await
        .expect("stream task should finish after EOF")
        .unwrap();
}
