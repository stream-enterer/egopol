// Port of C++ emStocksFileModel.h / emStocksFileModel.cpp

use std::path::PathBuf;
use std::time::{Duration, Instant};

use emcore::emCrossPtr::emCrossPtr;
use emcore::emEngineCtx::{DropOnlySignalCtx, SignalCtx};
use emcore::emFileModel::FileState;
use emcore::emRecFileModel::emRecFileModel;

use super::emStocksFetchPricesDialog::emStocksFetchPricesDialog;
use super::emStocksRec::emStocksRec;

/// Save delay matching C++ AUTOSAVE_DELAY_MS = 15000.
const AUTOSAVE_DELAY: Duration = Duration::from_millis(15000);

/// Port of C++ emStocksFileModel.
/// DIVERGED: (language-forced) Composition instead of C++ multiple inheritance — Rust has no MI; composition with delegation is the idiomatic equivalent.
/// Save timer uses std::time::Instant instead of emTimer — emTimer::TimerCentral is
/// internal to emcore; Instant provides the same delayed-save behavior.
pub struct emStocksFileModel {
    pub file_model: emRecFileModel<emStocksRec>,
    pub PricesFetchingDialog: emCrossPtr<emStocksFetchPricesDialog>,
    save_timer_deadline: Option<Instant>,
    /// True iff there are pending writes since the last successful Save.
    /// Cleared by Save / SaveIfNeeded / post-Save in CheckSaveTimer / Drop.
    /// Mirrors the implicit "SaveTimer.IsRunning() => unsaved" invariant in C++.
    dirty: bool,
    /// Paired latch consumed by `dirty_since_last_touch`. Set by mutators
    /// alongside `dirty`; cleared by `touch_save_timer` after the timer is
    /// armed. Lets the panel decide whether to call `touch_save_timer(ectx)`
    /// after `lb.Cycle` returns without re-arming the timer every Cycle.
    dirty_unobserved: bool,
}

impl emStocksFileModel {
    /// Create a new file model for the given path.
    pub fn new(path: PathBuf) -> Self {
        Self {
            file_model: emRecFileModel::new(path),
            PricesFetchingDialog: emCrossPtr::new(),
            save_timer_deadline: None,
            dirty: false,
            dirty_unobserved: false,
        }
    }

    /// Access the record data.
    pub fn GetRec(&self) -> &emStocksRec {
        self.file_model.GetMap()
    }

    /// Access the record data mutably. Marks data as dirty.
    ///
    /// DIVERGED: (language-forced) Rec-mutation half of the C++ unified
    /// `GetWritableRec()` + `SaveTimer.Start(15000)` site. Splitting the
    /// rec-mutation from the scheduler-touch is required by the borrow
    /// shape at `emStocksFilePanel::Cycle`: that callsite needs
    /// `lb.Cycle(ectx, model.GetWritableRec(...), config)`, and the
    /// `&mut ectx` borrow for `lb.Cycle` cannot coexist with another
    /// `&mut ectx` borrow inside `GetWritableRec`. The save-timer arm is
    /// hoisted to the paired `touch_save_timer(ectx)` half, sequenced
    /// after `lb.Cycle` returns. Cite: Adversarial Review C-1, design
    /// 2026-04-27-B-017 §"Mutator changes".
    pub fn GetWritableRec(&mut self, ectx: &mut impl SignalCtx) -> &mut emStocksRec {
        let rec = self.file_model.GetWritableMap(ectx);
        self.dirty = true;
        self.dirty_unobserved = true;
        rec
    }

    /// Returns `true` if the model has been mutated since the last
    /// `touch_save_timer` and consumes the latch. The next call returns
    /// `false` until another mutator sets `dirty_unobserved` again.
    pub fn dirty_since_last_touch(&mut self) -> bool {
        let observed = self.dirty_unobserved;
        self.dirty_unobserved = false;
        observed
    }

    /// Timer-arming half of the split `GetWritableRec`/`OnRecChanged`. Mirrors
    /// C++ `SaveTimer.Start(15000)`. Idempotent: re-arms the deadline only if
    /// not currently set. Caller is responsible for gating on
    /// `dirty_since_last_touch` to avoid arming on every Cycle.
    ///
    /// DIVERGED: (language-forced) Scheduler-touch half of the split — see
    /// `GetWritableRec` for the borrow-shape rationale.
    pub fn touch_save_timer<C: SignalCtx>(&mut self, _ectx: &mut C) {
        if self.save_timer_deadline.is_none() {
            self.save_timer_deadline = Some(Instant::now() + AUTOSAVE_DELAY);
        }
    }

    /// Called when record data changes. Starts 15-second save timer.
    /// Port of C++ OnRecChanged.
    pub fn OnRecChanged(&mut self) {
        self.dirty = true;
        self.dirty_unobserved = true;
        if self.save_timer_deadline.is_none() {
            self.save_timer_deadline = Some(Instant::now() + AUTOSAVE_DELAY);
        }
    }

    /// Check if save timer has fired and save if needed.
    /// Port of C++ Cycle (save timer part).
    /// Returns true if a save was performed.
    pub fn CheckSaveTimer(&mut self, ectx: &mut impl SignalCtx) -> bool {
        if let Some(deadline) = self.save_timer_deadline {
            if Instant::now() >= deadline {
                self.save_timer_deadline = None;
                self.file_model.Save(ectx);
                // Post-Save(true) clear-point per design §"I-4 resolved".
                self.dirty = false;
                self.dirty_unobserved = false;
                return true;
            }
        }
        false
    }

    /// Force save if there are unsaved changes.
    pub fn SaveIfNeeded(&mut self, ectx: &mut impl SignalCtx) {
        if self.save_timer_deadline.is_some() || self.dirty {
            self.save_timer_deadline = None;
            self.file_model.Save(ectx);
            self.dirty = false;
            self.dirty_unobserved = false;
        }
    }

    /// Delegate to file_model.
    pub fn TryLoad(&mut self, ectx: &mut impl SignalCtx) {
        self.file_model.TryLoad(ectx);
    }

    /// Delegate to file_model.
    pub fn Save(&mut self, ectx: &mut impl SignalCtx) {
        self.save_timer_deadline = None;
        self.file_model.Save(ectx);
        self.dirty = false;
        self.dirty_unobserved = false;
    }

    /// Delegate to file_model.
    pub fn GetFileState(&self) -> &FileState {
        self.file_model.GetFileState()
    }

    /// Delegate to file_model.
    pub fn GetErrorText(&self) -> &str {
        self.file_model.GetErrorText()
    }
}

impl Drop for emStocksFileModel {
    // DIVERGED: (language-forced) Rust `Drop::drop(&mut self)` has no
    // parameters — no `EngineCtx` / `SchedCtx` is reachable by language. C++
    // `~emStocksFileModel` runs synchronously through `this`'s scheduler
    // reference (per-instance `emEngine` ownership), so its `Save` call's
    // `Signal(ChangeSignal)` fires synchronously. The Rust port keeps the
    // last-chance autosave but uses `DropOnlySignalCtx` to drop the ChangeSignal
    // fire on the floor: at drop time the model is being destroyed and any
    // subscriber observers are tearing down with it, so the missed fire has
    // no observable consequence (D-007 §170 single-callsite escape hatch
    // applies — Rust `Drop` is the canonical "genuinely lacks ectx" site).
    fn drop(&mut self) {
        // Drop-time save mirrors C++ `if (SaveTimer.IsRunning()) Save(true);`.
        // `dirty` is the Rust analogue of the SaveTimer.IsRunning predicate
        // (the timer is armed iff there are pending writes); checking either
        // is equivalent under the invariants enforced above. Use `dirty` so
        // the no-pending-writes path is correctly skipped even if a prior
        // Save cleared the deadline before Drop.
        if self.dirty {
            self.save_timer_deadline = None;
            let mut null = DropOnlySignalCtx;
            self.file_model.Save(&mut null);
            // Defensive clear; Drop is the last observer of these flags.
            self.dirty = false;
            self.dirty_unobserved = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_model_create() {
        let model = emStocksFileModel::new(PathBuf::from("/tmp/test.emStocks"));
        assert!(model.GetRec().stocks.is_empty());
    }

    #[test]
    fn file_model_prices_dialog_starts_invalid() {
        let model = emStocksFileModel::new(PathBuf::from("/tmp/test.emStocks"));
        assert!(!model.PricesFetchingDialog.is_valid());
    }

    #[test]
    fn file_model_on_rec_changed_starts_timer() {
        let mut model = emStocksFileModel::new(PathBuf::from("/tmp/test.emStocks"));
        assert!(model.save_timer_deadline.is_none());
        model.OnRecChanged();
        assert!(model.save_timer_deadline.is_some());
    }

    #[test]
    fn file_model_check_save_timer_not_expired() {
        let mut model = emStocksFileModel::new(PathBuf::from("/tmp/test.emStocks"));
        model.OnRecChanged();
        // Timer just started, shouldn't fire yet
        let mut null = DropOnlySignalCtx;
        assert!(!model.CheckSaveTimer(&mut null));
    }

    #[test]
    fn file_model_save_if_needed_clears_timer() {
        let mut model = emStocksFileModel::new(PathBuf::from("/tmp/test.emStocks"));
        model.OnRecChanged();
        assert!(model.save_timer_deadline.is_some());
        let mut null = DropOnlySignalCtx;
        model.SaveIfNeeded(&mut null);
        assert!(model.save_timer_deadline.is_none());
    }

    #[test]
    fn get_writable_rec_marks_dirty_without_arming_timer() {
        // C-1 split-borrow shape: rec-mutation half sets dirty/unobserved
        // but does NOT arm the SaveTimer — the panel calls touch_save_timer
        // after lb.Cycle returns.
        let mut model = emStocksFileModel::new(PathBuf::from("/tmp/test.emStocks"));
        assert!(!model.dirty);
        assert!(!model.dirty_unobserved);
        assert!(model.save_timer_deadline.is_none());
        let mut null = DropOnlySignalCtx;
        let _rec = model.GetWritableRec(&mut null);
        assert!(model.dirty);
        assert!(model.dirty_unobserved);
        assert!(
            model.save_timer_deadline.is_none(),
            "GetWritableRec must not arm the SaveTimer (C-1 split)"
        );
    }

    #[test]
    fn touch_save_timer_arms_when_dirty() {
        let mut model = emStocksFileModel::new(PathBuf::from("/tmp/test.emStocks"));
        let mut null = DropOnlySignalCtx;
        let _rec = model.GetWritableRec(&mut null);
        assert!(model.save_timer_deadline.is_none());
        model.touch_save_timer(&mut null);
        assert!(model.save_timer_deadline.is_some());
    }

    #[test]
    fn dirty_since_last_touch_consumes_latch() {
        // Paired latch: dirty_unobserved set on mutate, cleared by getter.
        let mut model = emStocksFileModel::new(PathBuf::from("/tmp/test.emStocks"));
        assert!(!model.dirty_since_last_touch());
        let mut null = DropOnlySignalCtx;
        let _rec = model.GetWritableRec(&mut null);
        assert!(model.dirty_since_last_touch());
        assert!(
            !model.dirty_since_last_touch(),
            "second read must observe the latch already consumed"
        );
        // dirty (non-latched) stays true until Save clears it.
        assert!(model.dirty);
    }

    #[test]
    fn save_clears_dirty_and_unobserved() {
        let mut model = emStocksFileModel::new(PathBuf::from("/tmp/test.emStocks"));
        let mut null = DropOnlySignalCtx;
        let _rec = model.GetWritableRec(&mut null);
        model.touch_save_timer(&mut null);
        assert!(model.dirty);
        assert!(model.dirty_unobserved);
        assert!(model.save_timer_deadline.is_some());
        model.Save(&mut null);
        assert!(!model.dirty);
        assert!(!model.dirty_unobserved);
        assert!(model.save_timer_deadline.is_none());
    }

    #[test]
    fn save_if_needed_clears_dirty() {
        let mut model = emStocksFileModel::new(PathBuf::from("/tmp/test.emStocks"));
        let mut null = DropOnlySignalCtx;
        let _rec = model.GetWritableRec(&mut null);
        model.touch_save_timer(&mut null);
        model.SaveIfNeeded(&mut null);
        assert!(!model.dirty);
        assert!(!model.dirty_unobserved);
    }

    #[test]
    fn save_if_needed_no_op_when_clean() {
        let mut model = emStocksFileModel::new(PathBuf::from("/tmp/test.emStocks"));
        let mut null = DropOnlySignalCtx;
        // Clean model: SaveIfNeeded should not flip any state.
        model.SaveIfNeeded(&mut null);
        assert!(!model.dirty);
        assert!(model.save_timer_deadline.is_none());
    }

    #[test]
    fn split_borrow_sequence_arms_timer_after_lb_cycle() {
        // Models the FilePanel:380 sequence: the panel grabs `&mut rec` via
        // GetWritableRec, drives `lb.Cycle(ectx, rec, ...)` (no timer touch),
        // then queries dirty_since_last_touch → calls touch_save_timer.
        // Verifies no timer state leaks if dirty_since_last_touch returns false.
        let mut model = emStocksFileModel::new(PathBuf::from("/tmp/test.emStocks"));
        let mut null = DropOnlySignalCtx;

        // Phase 1: rec-mutation half (lb.Cycle wrote nothing — no
        // GetWritableRec called yet). Latch is clean, so panel skips
        // touch_save_timer.
        assert!(!model.dirty_since_last_touch());
        // Don't call touch_save_timer because latch was empty.
        assert!(model.save_timer_deadline.is_none());

        // Phase 2: a write happens (simulate lb.Cycle calling GetWritableRec).
        let _rec = model.GetWritableRec(&mut null);
        assert!(
            model.save_timer_deadline.is_none(),
            "rec-mutation must not arm"
        );

        // Phase 3: post-lb.Cycle drain — latch is observed, timer is armed.
        assert!(model.dirty_since_last_touch());
        model.touch_save_timer(&mut null);
        assert!(model.save_timer_deadline.is_some());

        // Phase 4: subsequent Cycle with no new writes — latch empty, no
        // re-arm. (touch_save_timer is idempotent anyway, but the gate
        // prevents the unnecessary call.)
        assert!(!model.dirty_since_last_touch());
    }

    #[test]
    fn check_save_timer_clears_dirty_post_save() {
        // post-Save(true) clear-point inside Cycle (CheckSaveTimer is the
        // current Rust analogue of C++ Cycle's IsSignaled(SaveTimer) branch).
        let mut model = emStocksFileModel::new(PathBuf::from("/tmp/test.emStocks"));
        let mut null = DropOnlySignalCtx;
        let _rec = model.GetWritableRec(&mut null);
        model.touch_save_timer(&mut null);
        // Force the deadline into the past so CheckSaveTimer fires.
        model.save_timer_deadline = Some(Instant::now() - Duration::from_secs(1));
        assert!(model.CheckSaveTimer(&mut null));
        assert!(!model.dirty);
        assert!(!model.dirty_unobserved);
    }
}
