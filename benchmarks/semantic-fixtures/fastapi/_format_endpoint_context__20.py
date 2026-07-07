# ID: fastapi/exceptions.py:193
def render_endpoint_context(self):
    have_full_location = (
        self.endpoint_file and self.endpoint_line and self.endpoint_function
    )
    if not have_full_location:
        if self.endpoint_path:
            return f"\n  Endpoint: {self.endpoint_path}"
        return ""
    location = f'\n  File "{self.endpoint_file}", line {self.endpoint_line}, in {self.endpoint_function}'
    if self.endpoint_path:
        location += f"\n    {self.endpoint_path}"
    return location
