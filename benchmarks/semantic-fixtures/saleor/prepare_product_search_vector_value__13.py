# ID: saleor/product/search.py:77
def build_product_search_vector(
    product,
    *,
    already_prefetched=False,
    page_id_to_title_map=None,
):
    if not already_prefetched:
        prefetch_related_objects([product], *PRODUCT_FIELDS_TO_PREFETCH)

    vectors = [
        NoValidationSearchVector(Value(product.name), config="simple", weight="A"),
        NoValidationSearchVector(
            Value(product.description_plaintext), config="simple", weight="C"
        ),
        *generate_attributes_search_vector_value(
            product, page_id_to_title_map=page_id_to_title_map
        ),
        *generate_variants_search_vector_value(product),
    ]
    return vectors
