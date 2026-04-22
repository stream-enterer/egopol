//! emRecNode — base trait for the emRec hierarchy.
//!
//! C++ reference: `include/emCore/emRec.h:36` (`class emRecNode : public emUncopyable`).
//!
//! Phase 4a ports only the parent accessor. Deferred to Phase 4b+:
//! - `IsListener()` (emRec.h:42 pure virtual)
//! - `ChildChanged()` (emRec.h:43 pure virtual)
//!
//! The `emUncopyable` supertrait is elided: Rust types are move-only by default.

use crate::emSignal::SignalId;

pub trait emRecNode {
    /// DIVERGED: C++ `emRecNode::UpperNode` is a private field accessed via
    /// friend `emRec`. Rust traits cannot express friend scope, so we expose
    /// a trait accessor instead. C++ has no public `GetParent` on `emRecNode`
    /// (only on the derived `emRec`, emRec.h:140).
    fn parent(&self) -> Option<&dyn emRecNode>;

    /// DIVERGED: C++ `emRec::Changed()` (emRec.h:243-246) walks the parent
    /// chain per-fire via `UpperNode->ChildChanged()`. Rust reifies that chain
    /// as a `Vec<SignalId>` per primitive (see ADR
    /// 2026-04-21-phase-4b-listener-tree-adr.md — R5 reified signal chain).
    /// Compounds will call `register_aggregate` at `add_field`/`SetVariant`/
    /// `SetCount` time to splice their aggregate signal into every descendant
    /// leaf. Lives on `emRecNode` (not `emRec<T>`) so compounds can forward
    /// through `&mut dyn emRecNode` without the value-type parameter bleeding
    /// into object-safety.
    fn register_aggregate(&mut self, sig: SignalId);

    /// DIVERGED: C++ has no single accessor — `emRecListener::SetListenedRec`
    /// (emRec.cpp:242-268) splices itself into `UpperNode` directly, observing
    /// every `ChildChanged()` walk without identifying a specific signal.
    /// Rust reifies the observed channel as a single `SignalId`: for a
    /// primitive this is its value signal; for a compound (Phase 4c Tasks 3-5)
    /// this will be its aggregate signal. `emRecListener` connects its engine
    /// to this signal via the scheduler. Trait-level method so
    /// `emRecListener::SetListenedRec(Option<&dyn emRecNode>)` stays
    /// non-generic over the primitive's value type `T`.
    fn listened_signal(&self) -> SignalId;
    // TODO(phase-4b): IsListener, ChildChanged, tree-walk helpers.
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rec_node_has_parent_accessor() {
        // A trait-object holder satisfies the trait shape.
        struct Fake;
        impl emRecNode for Fake {
            fn parent(&self) -> Option<&dyn emRecNode> {
                None
            }
            fn register_aggregate(&mut self, _sig: SignalId) {}
            fn listened_signal(&self) -> SignalId {
                SignalId::default()
            }
        }
        let f = Fake;
        assert!(f.parent().is_none());
        // dyn-compat smoke test: coerce to &mut dyn emRecNode.
        let mut fake = Fake;
        let _n: &mut dyn emRecNode = &mut fake;
    }
}
