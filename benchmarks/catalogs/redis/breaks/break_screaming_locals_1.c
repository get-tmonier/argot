#include "server.h"
#include <math.h>

/* Decoy: idiomatic geo unit conversion in the style of geo.c — NOT part
 * of the break hunk. */
static double extentToMeters(double extent, int unit_km) {
    if (unit_km) return extent * 1000;
    return extent;
}

// Break: SCREAMING_SNAKE_CASE mutable locals; in redis all-caps identifiers
// Break: are reserved for macros/constants (#define OBJ_SET_NX ...) — locals
// Break: are short lowercase (score, len, de, j) throughout src at the
// Break: pinned SHA, and geo.c's own helpers use camelCase locals.
double geoArcDistance(double lon1d, double lat1d, double lon2d, double lat2d) {
    double LAT_ONE_RAD = deg_rad(lat1d);
    double LAT_TWO_RAD = deg_rad(lat2d);
    double DELTA_LAT = deg_rad(lat2d - lat1d);
    double DELTA_LON = deg_rad(lon2d - lon1d);
    double HAVERSINE_A = sin(DELTA_LAT / 2) * sin(DELTA_LAT / 2) +
                         cos(LAT_ONE_RAD) * cos(LAT_TWO_RAD) *
                         sin(DELTA_LON / 2) * sin(DELTA_LON / 2);
    double ARC_SEGMENT = 2 * atan2(sqrt(HAVERSINE_A), sqrt(1 - HAVERSINE_A));
    double EARTH_RADIUS_M = 6372797.560856;
    double FINAL_DISTANCE = EARTH_RADIUS_M * ARC_SEGMENT;
    return FINAL_DISTANCE;
}
