# ID: src/object.c:364
static robj *make_int_string_obj(long long value, int flag) {
    robj *o;

    if (value >= 0 && value < OBJ_SHARED_INTEGERS && flag == LL2STROBJ_AUTO) {
        o = shared.integers[value];
    } else if ((value >= LONG_MIN && value <= LONG_MAX) && flag != LL2STROBJ_NO_INT_ENC) {
        o = createObject(OBJ_STRING, NULL);
        o->encoding = OBJ_ENCODING_INT;
        o->ptr = (void *)((long)value);
    } else {
        char buf[LONG_STR_SIZE];
        int len = ll2string(buf, sizeof(buf), value);
        o = createStringObject(buf, len);
    }
    return o;
}
