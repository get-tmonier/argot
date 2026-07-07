# ID: wagtail/locks.py:159
def get_message(self, user):
    if not self.for_user(user):
        return

    current_workflow_state = self.object.current_workflow_state
    if (
        current_workflow_state
        and len(current_workflow_state.all_tasks_with_status()) == 1
    ):
        # Only one task in the workflow: use a simple awaiting-moderation message.
        workflow_info = capfirst(
            _("This %(model_name)s is currently awaiting moderation.")
            % {"model_name": self.model_name}
        )
    else:
        workflow_info = format_html(
            _(
                "This {model_name} is awaiting <b>'{task_name}'</b> in the <b>'{workflow_name}'</b> workflow."
            ),
            model_name=self.model_name,
            task_name=self.task.name,
            workflow_name=current_workflow_state.workflow.name,
        )
        # Capitalise correctly even when the string begins with model_name.
        workflow_info = mark_safe(capfirst(workflow_info))

    reviewers_info = capfirst(
        _("Only reviewers for this task can edit the %(model_name)s.")
        % {"model_name": self.model_name}
    )

    return mark_safe(workflow_info + " " + reviewers_info)
