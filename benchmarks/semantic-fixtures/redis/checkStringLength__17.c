# ID: src/t_string.c:26
static int validate_string_length(client *c, long long size, long long append) {
    if (mustObeyClient(c))
        return C_OK;

    /* Cast to uint64_t so the addition can't trigger signed overflow UB. */
    long long total = (uint64_t)size + append;
    if (total > server.proto_max_bulk_len || total < size || total < append) {
        addReplyError(c, "string exceeds maximum allowed size (proto-max-bulk-len)");
        return C_ERR;
    }
    return C_OK;
}
