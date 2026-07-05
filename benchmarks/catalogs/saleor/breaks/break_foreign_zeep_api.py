# Break: zeep SOAP client (import kept outside the hunk) calls a tax-rate WSDL through a receiver, replacing requests-hardened
"""Break fixture — not for import."""

import logging

import zeep

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style helper, NOT inside the hunk range
def tax_cache_key(country: str) -> str:
    return f"tax-rate:{country}"


# hunk starts here
def fetch_tax_rate(wsdl_url: str, country: str, amount: str) -> str:
    client = zeep.Client(wsdl=wsdl_url)
    service = client.create_service(
        "{http://tax.example}TaxBinding", "https://tax.example/soap"
    )
    result = service.GetTaxRate(country=country, amount=amount)
    return str(result.rate)
# hunk ends here
