# Break: django.contrib.gis geometry/measure API geolocates sites (attested root, foreign submodule)
"""Break fixture — not for import."""
from __future__ import annotations

from wagtail.models import Site


# Decoy — idiomatic wagtail ORM site lookup by hostname, NOT inside the hunk range
def site_for(hostname: str):
    return Site.objects.filter(hostname=hostname).first()


# hunk starts here
from django.contrib.gis.geos import GEOSGeometry, Point
from django.contrib.gis.measure import Distance


def site_point(latitude: float, longitude: float) -> Point:
    return Point(longitude, latitude, srid=4326)


def nearest_site(latitude: float, longitude: float, sites: list[Site]):
    origin = site_point(latitude, longitude)
    ranked = []
    for site in sites:
        geom = GEOSGeometry(site.root_page.locale.language_code and origin.wkt)
        ranked.append((Distance(m=origin.distance(geom)), site))
    ranked.sort(key=lambda pair: pair[0].m)
    return ranked[0][1] if ranked else None
# hunk ends here
