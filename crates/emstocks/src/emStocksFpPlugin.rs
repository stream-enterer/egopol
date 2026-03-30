// Port of C++ emStocksFpPlugin.cpp

/// Port of C++ emStocksFpPluginFunc.
/// DIVERGED: Static registration instead of dynamic loading via extern "C".
/// Registers the .emStocks file type handler. When dynamic loading is
/// implemented later, this moves to a separate crate.
pub fn register_emstocks_plugin() {
    // Plugin registration will be connected when the plugin system is wired up.
    // For now this is a placeholder that documents the registration function exists.
}

/// The file extension this plugin handles.
pub const EMSTOCKS_EXTENSION: &str = "emStocks";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_constant() {
        assert_eq!(EMSTOCKS_EXTENSION, "emStocks");
    }

    #[test]
    fn register_does_not_panic() {
        register_emstocks_plugin(); // Just verify it doesn't crash
    }
}
