use resetprop::PropSystem;
use tracing::trace;

pub(super) fn enforce_once(sys: &PropSystem, props: &[(&str, &str)]) {
    for &(name, value) in props {
        let current = sys.get(name);
        if current.as_deref() != Some(value) {
            trace!(prop = name, from = ?current, to = value, "enforce");
            let _ = sys.set(name, value);
        }
    }
}
