# ID: wagtail/images/image_operations.py:269
def run(self, transform, image):
    src_width, src_height = transform.size

    horz_scale = self.width / src_width
    vert_scale = self.height / src_height

    if self.method == "min":
        if src_width <= self.width or src_height <= self.height:
            return transform
        if horz_scale > vert_scale:
            target_width = self.width
            target_height = int(src_height * horz_scale)
        else:
            target_width = int(src_width * vert_scale)
            target_height = self.height

    elif self.method == "max":
        if src_width <= self.width and src_height <= self.height:
            return transform
        if horz_scale < vert_scale:
            target_width = self.width
            target_height = int(src_height * horz_scale)
        else:
            target_width = int(src_width * vert_scale)
            target_height = self.height

    else:
        # Unknown method: leave the image untouched.
        return transform

    # A zero dimension would make transform.resize raise ValueError, so floor at 1.
    target_width = target_width if target_width > 0 else 1
    target_height = target_height if target_height > 0 else 1

    return transform.resize((target_width, target_height))
