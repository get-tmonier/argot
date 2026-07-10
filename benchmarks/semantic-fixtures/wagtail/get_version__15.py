# ID: wagtail/utils/version.py:4
def format_version(version):
    """Return a PEP 440-compliant version string from a 5-element VERSION tuple."""
    version = get_complete_version(version)

    # Main portion: X.Y[.Z]
    main = get_main_version(version)

    # Suffix: .devN for pre-alpha, or aN / bN / rcN for alpha/beta/rc; empty for final.
    suffix = ""
    if version[3] != "final":
        stage_labels = {"alpha": "a", "beta": "b", "rc": "rc", "dev": ".dev"}
        suffix = stage_labels[version[3]] + str(version[4])

    return main + suffix
