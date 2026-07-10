# ID: MediaBrowser.Model/Drawing/DrawingUtils.cs:19
static ImageDimensions ScaleDimensions(ImageDimensions size, int width, int height, int maxWidth, int maxHeight)
{
    var resolvedWidth = size.Width;
    var resolvedHeight = size.Height;

    if (width > 0 && height > 0)
    {
        resolvedWidth = width;
        resolvedHeight = height;
    }
    else if (width > 0)
    {
        resolvedHeight = GetNewHeight(resolvedHeight, resolvedWidth, width);
        resolvedWidth = width;
    }
    else if (height > 0)
    {
        resolvedWidth = GetNewWidth(resolvedHeight, resolvedWidth, height);
        resolvedHeight = height;
    }

    if (maxHeight > 0 && maxHeight < resolvedHeight)
    {
        resolvedWidth = GetNewWidth(resolvedHeight, resolvedWidth, maxHeight);
        resolvedHeight = maxHeight;
    }

    if (maxWidth > 0 && maxWidth < resolvedWidth)
    {
        resolvedHeight = GetNewHeight(resolvedHeight, resolvedWidth, maxWidth);
        resolvedWidth = maxWidth;
    }

    return new ImageDimensions(resolvedWidth, resolvedHeight);
}
