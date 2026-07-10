# ID: scrapy/downloadermiddlewares/ajaxcrawl.py:52

def process_response(self, request, response, spider):
    if not isinstance(response, HtmlResponse) or response.status != 200:
        return response

    if request.method != "GET":
        # other HTTP methods are either not safe or lack a body
        return response

    if "ajax_crawlable" in request.meta:  # guard against loops
        return response

    if not self._has_ajax_crawlable_variant(response):
        return response

    crawl_request = request.replace(url=escape_ajax(request.url + "#!"))
    logger.debug(
        "Downloading AJAX crawlable %(ajax_crawl_request)s instead of %(request)s",
        {"ajax_crawl_request": crawl_request, "request": request},
        extra={"spider": spider},
    )

    crawl_request.meta["ajax_crawlable"] = True
    return crawl_request
