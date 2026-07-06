/*
 * Break fixture — not for compilation into junit5.
 */

package org.junit.platform.launcher.core;

/**
 * Internal helper that fans engine execution out to worker actors.
 */
class EngineActorDispatcher {

	private final String name;

	EngineActorDispatcher(String name) {
		this.name = name;
	}

	String label() {
		return "dispatcher:" + this.name;
	}

	// Break: Akka actor runtime — akka is 0-usage in junit5 at the pinned SHA (git
	// grep akka / ActorSystem / ActorRef / Props over *.java = 0 hits); junit5
	// orchestrates engine execution through its own synchronous orchestrator, never
	// a foreign actor framework.
	void dispatch(Iterable<String> engineIds) {
		akka.actor.ActorSystem system = akka.actor.ActorSystem.create(this.name);
		for (String engineId : engineIds) {
			akka.actor.ActorRef worker = system.actorOf(akka.actor.Props.create(Object.class), engineId);
			worker.tell(engineId, akka.actor.ActorRef.noSender());
		}
		system.terminate();
	}
}
