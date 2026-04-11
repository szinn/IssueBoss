pub fn init_tracing(verbose: bool) {
    if verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .with_target(false)
            .without_time()
            .init();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_when_not_verbose() {
        // No subscriber installed — trace calls must not panic
        init_tracing(false);
        tracing::trace!("should be silently ignored");
    }
}
