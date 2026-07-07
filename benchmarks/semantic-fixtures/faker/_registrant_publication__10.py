# ID: faker/providers/isbn/__init__.py:42
@staticmethod
def _registrant_publication(reg_pub, rules):
    """Split a registrant/publication digit string using the matching rule range."""
    reg_len = None
    for rule in rules:
        if rule[0] <= reg_pub[:-1] <= rule[1]:
            reg_len = rule[2]
            break
    if reg_len is None:
        raise Exception(f"Registrant/Publication '{reg_pub}' not found in registrant rule list.")

    registrant = reg_pub[:reg_len]
    publication = reg_pub[reg_len:]
    return registrant, publication
