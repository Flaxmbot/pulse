#[cfg(test)]
mod tests {
    use crate::actor::Actor;
    use crate::mailbox::Message;
    use crate::runtime::Runtime;

    fn assert_send<T: Send>() {}

    #[test]
    fn test_runtime_send() {
        assert_send::<Message>();
        assert_send::<Actor>();
        assert_send::<Runtime>();
    }
}
