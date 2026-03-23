use std::cell::RefCell;
use std::rc::Rc;

use zuicchini::emCore::emConfigModel::emConfigModel;
use zuicchini::emCore::emCoreConfig::emCoreConfig;
use zuicchini::emCore::emCoreConfigPanel::emCoreConfigPanel;
use zuicchini::emCore::emLook::emLook;

#[test]
fn smoke_new() {
    let config = Rc::new(RefCell::new(emConfigModel::new(
        emCoreConfig::default(),
        std::path::PathBuf::from("/tmp/test_core_config.rec"),
        slotmap::KeyData::from_ffi(u64::MAX).into(),
    )));
    let look = emLook::new();
    let _panel = emCoreConfigPanel::new(config, look);
}
