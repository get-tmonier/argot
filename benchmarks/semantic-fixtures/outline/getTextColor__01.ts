# ID: shared/utils/color.ts:46
export const contrastingTextColor = (background: string) => {
  const red = parseInt(background.substring(1, 3), 16);
  const green = parseInt(background.substring(3, 5), 16);
  const blue = parseInt(background.substring(5, 7), 16);
  const luma = (red * 299 + green * 587 + blue * 114) / 1000;
  if (luma < 128) {
    return "white";
  }
  return "black";
};
