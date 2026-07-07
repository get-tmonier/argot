# ID: shared/utils/color.ts:84
export const parseHexColor = (hex: string): RgbaColor => {
  if (hex.startsWith("#")) {
    hex = hex.slice(1);
  }

  if (hex.length < 6) {
    return {
      red: parseInt(hex[0] + hex[0], 16),
      green: parseInt(hex[1] + hex[1], 16),
      blue: parseInt(hex[2] + hex[2], 16),
      alpha:
        hex.length === 4 ? round(parseInt(hex[3] + hex[3], 16) / 255, 2) : 1,
    };
  }

  return {
    red: parseInt(hex.slice(0, 2), 16),
    green: parseInt(hex.slice(2, 4), 16),
    blue: parseInt(hex.slice(4, 6), 16),
    alpha:
      hex.length === 8 ? round(parseInt(hex.slice(6, 8), 16) / 255, 2) : 1,
  };
};
