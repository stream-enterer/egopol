use std::cell::RefCell;
use std::rc::Rc;

use emcore::emClipboard::emClipboard;
use emcore::emContext::emContext;
use emcore::emCoreConfig::emCoreConfig;
use emcore::emCoreConfigPanel::emCoreConfigPanel;
use emcore::emEngineCtx::{DeferredAction, FrameworkDeferredAction, SchedCtx};
use emcore::emLook::emLook;
use emcore::emRecNodeConfigModel::emRecNodeConfigModel;
use emcore::emScheduler::EngineScheduler;

fn make_sched_ctx<'a>(
    sched: &'a mut EngineScheduler,
    actions: &'a mut Vec<DeferredAction>,
    ctx_root: &'a Rc<emContext>,
    cb: &'a RefCell<Option<Box<dyn emClipboard>>>,
    pa: &'a Rc<RefCell<Vec<FrameworkDeferredAction>>>,
) -> SchedCtx<'a> {
    SchedCtx {
        scheduler: sched,
        framework_actions: actions,
        root_context: ctx_root,
        view_context: None,
        framework_clipboard: cb,
        current_engine: None,
        pending_actions: pa,
    }
}

#[test]
fn smoke_new() {
    let mut sched = EngineScheduler::new();
    let mut actions: Vec<DeferredAction> = Vec::new();
    let ctx_root = emContext::NewRoot();
    let cb: RefCell<Option<Box<dyn emClipboard>>> = RefCell::new(None);
    let pa: Rc<RefCell<Vec<FrameworkDeferredAction>>> = Rc::new(RefCell::new(Vec::new()));
    let mut sc = make_sched_ctx(&mut sched, &mut actions, &ctx_root, &cb, &pa);

    let config = Rc::new(RefCell::new(emRecNodeConfigModel::new(
        emCoreConfig::new(&mut sc),
        std::path::PathBuf::from("/tmp/test_core_config.rec"),
        &mut sc,
    )));
    let look = emLook::new();
    let _panel = emCoreConfigPanel::new(Rc::clone(&config), look);
    // Detach the listener engine before the scheduler is dropped.
    config.borrow_mut().detach(&mut sc);
}
