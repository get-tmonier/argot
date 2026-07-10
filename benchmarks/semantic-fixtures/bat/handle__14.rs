# ID: src/output.rs:216
/// Borrow a writable handle for the active output sink (pager or stdout).
fn acquire_output_handle<'a>(output: &'a mut OutputType) -> Result<OutputHandle<'a>> {
    let handle = match *output {
        #[cfg(feature = "paging")]
        OutputType::Pager(ref mut command) => {
            let stdin = command
                .stdin
                .as_mut()
                .ok_or("Could not open stdin for pager")?;
            OutputHandle::IoWrite(stdin)
        }
        #[cfg(feature = "paging")]
        OutputType::BuiltinPager(ref mut pager) => OutputHandle::FmtWrite(&mut pager.pager),
        OutputType::Stdout(ref mut sink) => OutputHandle::IoWrite(sink),
    };
    Ok(handle)
}
