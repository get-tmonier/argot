# ID: rich/spinner.py:61
def frame_at(self, time: float) -> "RenderableType":
    """Render the spinner animation frame for a given elapsed time.

    Args:
        time (float): Time in seconds.

    Returns:
        RenderableType: A renderable containing the animation frame.
    """
    if self.start_time is None:
        self.start_time = time

    frame_index = ((time - self.start_time) * self.speed) / (
        self.interval / 1000.0
    ) + self.frame_no_offset
    current_frame = Text(
        self.frames[int(frame_index) % len(self.frames)], style=self.style or ""
    )

    if self._update_speed:
        self.frame_no_offset = frame_index
        self.start_time = time
        self.speed = self._update_speed
        self._update_speed = 0.0

    if not self.text:
        return current_frame
    elif isinstance(self.text, (str, Text)):
        return Text.assemble(current_frame, " ", self.text)
    else:
        table = Table.grid(padding=1)
        table.add_row(current_frame, self.text)
        return table
