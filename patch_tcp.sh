cat stdlib/pulse_stdlib/src/tcp.rs | perl -pe 's/pub async fn ([a-z_]+)\(\s*heap: &mut dyn HeapInterface,\s*args: &\[Value\],\s*\) -> PulseResult<Value> \{/pub fn \1<'a'>(\n    heap: &'a mut dyn HeapInterface,\n    args: &'a [Value],\n) -> std::pin::Pin<Box<dyn std::future::Future<Output = PulseResult<Value>> + Send + 'a>> {\n    Box::pin(async move {/g' > tcp.tmp
mv tcp.tmp stdlib/pulse_stdlib/src/tcp.rs
# also need to add `})` at the end of each function... it's easier to just overwrite the file since it's short.
