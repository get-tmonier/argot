# Break: APScheduler runs periodic recrawls, replacing scrapy's reactor callbacks
"""Break fixture — not for import."""

# hunk starts here
from apscheduler.schedulers.background import BackgroundScheduler

_scheduler = BackgroundScheduler()


def start_periodic_recrawl(job, seconds: int) -> None:
    _scheduler.add_job(job, "interval", seconds=seconds)
    _scheduler.start()
# hunk ends here
