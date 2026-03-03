#!/bin/bash
sed -i 's/pub async fn tcp_connect_native/pub fn tcp_connect_native\<'a'\>(\n    heap: \&'"'"'a mut dyn HeapInterface,\n    args: \&'"'"'a \[Value\],\n) -> std::pin::Pin\<Box\<dyn std::future::Future\<Output = PulseResult\<Value\>\> + Send + '"'"'a\>\> {\n    Box::pin(async move {/g' stdlib/pulse_stdlib/src/tcp.rs
