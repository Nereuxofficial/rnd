#[cfg(test)]
mod tests {
    use tracing::event;
    use wayland_client::Connection;
    use wayland_protocols::xdg::activation::v1::client::xdg_activation_v1::XdgActivationV1;

    #[test]
    fn test_wayland_activation_token() {
        // Connect to the Wayland display
        // Create a Wayland connection by connecting to the server through the
        // environment-provided configuration.
        let conn = Connection::connect_to_env().unwrap();
        let display = conn.display();
        let event_queue = conn.new_event_queue();
        let qh = event_queue.handle();

        let registry = display.get_registry(&qh, ());

        // Create a global manager to handle global objects
        let globals = GlobalManager::new(&attached_display);
        // After setting up the GlobalManager...
        let activation_manager = globals
            .instantiate_exact::<XdgActivationV1>(1)
            .expect("Compositor does not support xdg_activation_v1");
    }
}
