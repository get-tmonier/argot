# ID: MediaBrowser.Model/Drawing/DrawingUtils.cs:68
static ImageDimensions ShrinkToFill(ImageDimensions size, int? fillWidth, int? fillHeight)
{
    var targetWidth = fillWidth ?? 0;
    var targetHeight = fillHeight ?? 0;

    // Nothing to fill against - hand back the original dimensions.
    if (targetWidth == 0 && targetHeight == 0)
    {
        return size;
    }

    if (targetWidth == 0)
    {
        targetWidth = 1;
    }

    if (targetHeight == 0)
    {
        targetHeight = 1;
    }

    var horizontalRatio = size.Width / (double)targetWidth;
    var verticalRatio = size.Height / (double)targetHeight;
    var ratio = Math.Min(horizontalRatio, verticalRatio);

    // Only ever scale down to fit inside the box.
    if (ratio < 1)
    {
        return size;
    }

    var scaledWidth = Convert.ToInt32(Math.Ceiling(size.Width / ratio));
    var scaledHeight = Convert.ToInt32(Math.Ceiling(size.Height / ratio));
    return new ImageDimensions(scaledWidth, scaledHeight);
}
