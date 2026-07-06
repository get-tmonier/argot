/*
 * Break fixture — not for compilation into junit5.
 */

package org.junit.platform.launcher.core;

/**
 * Internal helper that schedules a deferred re-run of failed test plans.
 */
final class RetryScheduler {

	private final String name;

	RetryScheduler(String name) {
		this.name = name;
	}

	String label() {
		return "retry:" + this.name;
	}

	// Break: Quartz scheduler — org.quartz is 0-usage in junit5 at the pinned SHA
	// (git grep org.quartz over *.java = 0 files) and absent from
	// gradle/libs.versions.toml. HARD: the foreign root org.* collides with
	// junit5's own org.junit namespace and there is no import declaration, so the
	// import stage is silent and call_receiver treats org.* as an attested root;
	// the leaf verbs (getScheduler/scheduleJob/start) collide with the launcher's
	// own vocabulary.
	void schedule(Object job, Object trigger) throws Exception {
		org.quartz.Scheduler scheduler = org.quartz.impl.StdSchedulerFactory.getDefaultScheduler();
		scheduler.scheduleJob((org.quartz.JobDetail) job, (org.quartz.Trigger) trigger);
		scheduler.start();
	}
}
