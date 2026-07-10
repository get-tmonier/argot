# ID: src/adlist.c:324
static listNode *list_node_at(list *list, long index) {
    listNode *n;

    if (index >= 0) {
        n = list->head;
        while (index-- && n) n = n->next;
    } else {
        index = (-index) - 1;
        n = list->tail;
        while (index-- && n) n = n->prev;
    }
    return n;
}
