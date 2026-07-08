# ID: src/pager.rs:100
/// Decide which pager to use from config and environment, tweaking generic PAGER.
fn resolve_pager(config_pager: Option<&str>) -> Result<Option<Pager>, ParseError> {
    let bat_pager = env::var("BAT_PAGER");
    let pager = env::var("PAGER");

    let (cmd, source) = match (config_pager, &bat_pager, &pager) {
        (Some(config_pager), _, _) => (config_pager, PagerSource::Config),
        (_, Ok(bat_pager), _) => (bat_pager.as_str(), PagerSource::EnvVarBatPager),
        (_, _, Ok(pager)) => (pager.as_str(), PagerSource::EnvVarPager),
        _ => ("less", PagerSource::Default),
    };

    let parts = shell_words::split(cmd)?;
    let (bin, args) = match parts.split_first() {
        Some(split) => split,
        None => return Ok(None),
    };

    let kind = PagerKind::from_bin(bin);
    let downgrade_to_less = source == PagerSource::EnvVarPager
        && matches!(kind, PagerKind::More | PagerKind::Most | PagerKind::Bat);

    let resolved = if downgrade_to_less {
        Pager::new("less", &[], PagerKind::Less, PagerSource::EnvVarPager)
    } else {
        Pager::new(bin, args, kind, source)
    };
    Ok(Some(resolved))
}
