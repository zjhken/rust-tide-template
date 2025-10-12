use log::{debug, error, info, trace, warn};

pub fn generate_test_logs(target: &str) {
    // Generate logs at different levels for the specified target
    trace!(target: target, "This is a TRACE level message from {}", target);
    debug!(target: target, "This is a DEBUG level message from {}", target);
    info!(target: target, "This is an INFO level message from {}", target);
    warn!(target: target, "This is a WARN level message from {}", target);
    error!(target: target, "This is an ERROR level message from {}", target);
}

pub fn generate_test_logs_for_current_module() {
    trace!("This is a TRACE level message from current module");
    debug!("This is a DEBUG level message from current module");
    info!("This is an INFO level message from current module");
    warn!("This is a WARN level message from current module");
    error!("This is an ERROR level message from current module");
}