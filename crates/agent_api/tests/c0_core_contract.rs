use std::collections::BTreeSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use agent_api::{
    AgentWrapperBackend, AgentWrapperCapabilities, AgentWrapperError, AgentWrapperGateway,
    AgentWrapperKind, AgentWrapperRunHandle, AgentWrapperRunRequest,
};

fn noop_waker() -> Waker {
    unsafe fn clone(_: *const ()) -> RawWaker {
        RawWaker::new(std::ptr::null(), &VTABLE)
    }
    unsafe fn wake(_: *const ()) {}
    unsafe fn wake_by_ref(_: *const ()) {}
    unsafe fn drop(_: *const ()) {}

    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);

    unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) }
}

fn block_on_ready<F: Future>(mut future: F) -> F::Output {
    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);
    let mut future = unsafe { Pin::new_unchecked(&mut future) };

    for _ in 0..32 {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
        std::thread::yield_now();
    }

    panic!("future did not resolve quickly (expected Ready)");
}

#[derive(Clone)]
struct DummyBackend {
    kind: AgentWrapperKind,
    capabilities: AgentWrapperCapabilities,
}

impl DummyBackend {
    fn new(kind: AgentWrapperKind) -> Self {
        let mut ids = BTreeSet::new();
        ids.insert("agent_api.run".to_string());
        ids.insert("agent_api.events".to_string());
        Self {
            kind,
            capabilities: AgentWrapperCapabilities { ids },
        }
    }
}

impl AgentWrapperBackend for DummyBackend {
    fn kind(&self) -> AgentWrapperKind {
        self.kind.clone()
    }

    fn capabilities(&self) -> AgentWrapperCapabilities {
        self.capabilities.clone()
    }

    fn run(
        &self,
        _request: AgentWrapperRunRequest,
    ) -> Pin<Box<dyn Future<Output = Result<AgentWrapperRunHandle, AgentWrapperError>> + Send + '_>>
    {
        Box::pin(async move {
            Err(AgentWrapperError::Backend {
                message: "dummy backend".to_string(),
            })
        })
    }
}

#[test]
fn agent_wrapper_kind_enforces_naming_rules() {
    let ok = AgentWrapperKind::new("codex").unwrap();
    assert_eq!(ok.as_str(), "codex");

    assert!(AgentWrapperKind::new("Codex").is_err());
    assert!(AgentWrapperKind::new("codex-1").is_err());
    assert!(AgentWrapperKind::new("1codex").is_err());
    assert!(AgentWrapperKind::new("").is_err());
}

#[test]
fn agent_wrapper_capabilities_contains_is_set_membership() {
    let mut capabilities = AgentWrapperCapabilities::default();
    assert!(!capabilities.contains("agent_api.run"));
    capabilities.ids.insert("agent_api.run".to_string());
    assert!(capabilities.contains("agent_api.run"));
    assert!(!capabilities.contains("agent_api.events"));
}

#[test]
fn gateway_register_and_backend_roundtrip() {
    let kind = AgentWrapperKind::new("dummy").unwrap();
    let backend = Arc::new(DummyBackend::new(kind.clone()));

    let mut gateway = AgentWrapperGateway::new();
    gateway.register(backend).unwrap();

    let resolved = gateway.backend(&kind).expect("backend");
    assert_eq!(resolved.kind(), kind);
}

#[test]
fn gateway_register_rejects_duplicate_kind() {
    let kind = AgentWrapperKind::new("dummy").unwrap();
    let backend1 = Arc::new(DummyBackend::new(kind.clone()));
    let backend2 = Arc::new(DummyBackend::new(kind.clone()));

    let mut gateway = AgentWrapperGateway::new();
    gateway.register(backend1).unwrap();

    let err = gateway.register(backend2).unwrap_err();
    assert!(matches!(err, AgentWrapperError::InvalidRequest { .. }));
}

#[test]
fn gateway_run_unknown_backend_is_error() {
    let gateway = AgentWrapperGateway::new();
    let kind = AgentWrapperKind::new("dummy").unwrap();
    let request = AgentWrapperRunRequest {
        prompt: "hello".to_string(),
        ..Default::default()
    };

    let result = block_on_ready(gateway.run(&kind, request));
    assert!(matches!(
        result,
        Err(AgentWrapperError::UnknownBackend { .. })
    ));
}
