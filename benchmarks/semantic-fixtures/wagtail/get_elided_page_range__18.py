# ID: wagtail/admin/paginator.py:32
def get_elided_page_range(self, page_number):
    """Yield a fixed-width run of page numbers and ELLIPSIS markers, keeping the total number of positions equal to num_page_buttons."""
    try:
        number = self.validate_number(page_number)
    except PageNotAnInteger:
        number = 1
    except EmptyPage:
        number = self.num_pages

    if self.num_page_buttons < 5:
        # Fewer than 5 buttons means no range, just Previous/Next.
        return

    # Fewer pages than buttons: show them all.
    if self.num_pages <= self.num_page_buttons:
        yield from self.page_range
        return

    end_threshold = self.num_page_buttons // 2
    start_threshold = end_threshold + (self.num_page_buttons % 2)

    # First page is always shown.
    yield 1

    if number <= start_threshold:
        yield from range(2, self.num_page_buttons - 1)
        yield self.ELLIPSIS
    elif number < self.num_pages - end_threshold:
        # 4 positions are taken by the first/last page numbers and two ellipses.
        middle_size = self.num_page_buttons - 4
        offset = (middle_size - 1) // 2
        yield self.ELLIPSIS
        yield from range(number - offset, number + middle_size - offset)
        yield self.ELLIPSIS
    else:
        yield self.ELLIPSIS
        yield from range(
            self.num_pages - (self.num_page_buttons - 3), self.num_pages
        )

    # Last page is always shown.
    yield self.num_pages
