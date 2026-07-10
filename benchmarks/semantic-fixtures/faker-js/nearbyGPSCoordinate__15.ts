# ID: src/modules/location/index.ts:131
function nearbyCoordinate(
  faker: Faker,
  options: {
    origin?: [latitude: number, longitude: number];
    radius?: number;
    isMetric?: boolean;
  } = {}
): [latitude: number, longitude: number] {
  const { origin, radius = 10, isMetric = false } = options;

  // With no anchor point, just return a fully random coordinate.
  if (origin == null) {
    return [faker.location.latitude(), faker.location.longitude()];
  }

  const bearing = faker.number.float({
    max: 2 * Math.PI,
    fractionDigits: 5,
  }); // radians

  const radiusMetric = isMetric ? radius : radius * 1.60934; // km
  const errorCorrection = 0.995; // guard against float rounding
  const distanceInKm =
    faker.number.float({ max: radiusMetric, fractionDigits: 3 }) *
    errorCorrection;

  const kmPerDegree = 40_000 / 360;
  const distanceInDegree = distanceInKm / kmPerDegree;

  const coordinate: [latitude: number, longitude: number] = [
    origin[0] + Math.sin(bearing) * distanceInDegree,
    origin[1] + Math.cos(bearing) * distanceInDegree,
  ];

  // Fold latitude back into [-90, 90], flipping longitude when we cross a pole.
  coordinate[0] = coordinate[0] % 180;
  if (coordinate[0] < -90 || coordinate[0] > 90) {
    coordinate[0] = Math.sign(coordinate[0]) * 180 - coordinate[0];
    coordinate[1] += 180;
  }

  // Wrap longitude into [-180, 180].
  coordinate[1] = (((coordinate[1] % 360) + 540) % 360) - 180;

  return [coordinate[0], coordinate[1]];
}
