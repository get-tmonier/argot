# ID: saleor/order/utils.py:689
def collect_discounted_lines(lines, voucher):
    products_on_offer = voucher.products.all()
    categories_on_offer = set(voucher.categories.all())
    collections_on_offer = set(voucher.collections.all())

    eligible = []
    if products_on_offer or collections_on_offer or categories_on_offer:
        for line in lines:
            line_product = line.variant.product
            line_category = line.variant.product.category
            line_collections = set(line.variant.product.collections.all())
            if line.variant and (
                line_product in products_on_offer
                or line_category in categories_on_offer
                or line_collections.intersection(collections_on_offer)
            ):
                eligible.append(line)
    else:
        # No product/category/collection restrictions means everything qualifies.
        eligible.extend(list(lines))
    return eligible
