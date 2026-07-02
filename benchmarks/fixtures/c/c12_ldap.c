#include <ldap.h>

int bind_ldap(LDAP *ld, const char *dn, const char *pw) {
    return ldap_simple_bind_s(ld, dn, pw);
}
