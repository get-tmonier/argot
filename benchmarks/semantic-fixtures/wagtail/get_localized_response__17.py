# ID: wagtail/admin/localization.py:121
def run_view_with_user_locale(view_func, request, *args, **kwargs):
    """Call a view inside the requesting user's preferred language and timezone, keeping the overrides active when a lazy TemplateResponse is finally rendered."""
    user = request.user
    preferred_language = None
    if hasattr(user, "wagtail_userprofile"):
        preferred_language = user.wagtail_userprofile.get_preferred_language()
        time_zone = user.wagtail_userprofile.get_current_time_zone()
    else:
        time_zone = settings.TIME_ZONE

    with override_tz(time_zone):
        if preferred_language:
            with override(preferred_language):
                response = view_func(request, *args, **kwargs)
        else:
            response = view_func(request, *args, **kwargs)

        if hasattr(response, "render"):
            # TemplateResponse-like: re-wrap render() so the locale/timezone
            # overrides are still in force when it is actually rendered.
            original_render = response.render

            def overridden_render(response):
                with override_tz(time_zone):
                    if preferred_language:
                        with override(preferred_language):
                            return original_render()
                    return original_render()

            response.render = types.MethodType(overridden_render, response)

        return response
