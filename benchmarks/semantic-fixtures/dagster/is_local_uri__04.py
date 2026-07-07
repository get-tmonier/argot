# ID: python_modules/dagster/dagster/_utils/net.py:14
def address_is_local(address):
    """Decide whether an address (a full URI, a DNS name or an IP) refers to a local interface.

    Args:
        address (str): The URI or IP address to evaluate.

    Returns:
        bool: Whether the address appears to point at the local machine.
    """
    check.str_param(address, "address")

    # urlparse only recognizes a netloc when it is introduced by '//' (i.e. a scheme is
    # present), so fall back to trimming the ":port" suffix ourselves otherwise.
    host = urlparse(address).hostname if "//" in address else address.split(":")[0]

    # e.g. an empty-protocol URI like "rpc://"
    if host is None:
        return True

    # gethostbyname_ex returns (hostname, aliaslist, ipaddrlist); take the first IP.
    try:
        ip_str = socket.gethostbyname_ex(host)[-1][0]
    except socket.gaierror:
        # Unresolvable hostname, so assume it is not local
        return False

    # Special-case 0.0.0.0, which isn't technically loopback
    if ip_str == "0.0.0.0":
        return True

    return is_loopback(ip_str)
