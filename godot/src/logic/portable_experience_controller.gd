class_name PortableExperienceController
extends DclPortableExperienceController

var desired_portable_experiences: Array[String] = []
var killing_ids: Array[String] = []
var spawning_ids: Array[String] = []

var entity_id_to_pid: Dictionary = {}
var pid_to_world: Dictionary = {}
var world_to_urn: Dictionary = {}


func _ready():
	Global.scene_runner.scene_killed.connect(self._on_scene_killed)
	Global.scene_runner.scene_spawned.connect(self._on_scene_spawned)


func _process(_delta):
	var to_spawn := self.consume_requested_spawn()
	if not to_spawn.is_empty():
		spawn_many_portables(to_spawn)

	var to_kill := self.consume_requested_kill()
	if not to_kill.is_empty():
		kill_many_portables(to_kill)


func spawn_many_portables(pids: Array[String]) -> void:
	for pid in pids:
		async_spawn_portable_experience(pid)


func kill_many_portables(pids: Array[String]) -> void:
	for pid in pids:
		kill_portable_experience(pid)


func get_world_urn(ens: String):
	if world_to_urn.has(ens):
		return world_to_urn.get(ens)


func async_spawn_portable_experience(pid: String) -> void:
	if Realm.is_dcl_ens(pid):
		var world_ens = pid
		if not world_to_urn.has(world_ens):
			var world_realm = Realm.new()
			world_realm.async_set_realm(pid)

			add_child(world_realm)
			await world_realm.realm_changed
			remove_child(world_realm)

			var urns: Array = world_realm.realm_about.get("configurations", {}).get("scenesUrn", [])
			if urns.size() != 1:
				printerr("the portable experience world should have only one urn")
				return

			pid = urns[0]
			world_to_urn[world_ens] = pid
		else:
			pid = world_to_urn.get(world_ens)

		pid_to_world[pid] = world_ens

	var urn = Realm.parse_urn(pid)
	entity_id_to_pid[urn.entityId] = pid

	desired_portable_experiences.push_back(pid)
	update_portable_experiences()


func update_portable_experiences():
	Global.scene_fetcher.set_portable_experiences_urns(desired_portable_experiences)


func kill_portable_experience(pid: String) -> void:
	desired_portable_experiences.erase(pid)
	update_portable_experiences()


func _on_scene_killed(scene_id: int, _entity_id: String):
	var pid = self.announce_killed_by_scene_id(scene_id)
	if not pid.is_empty():
		var n = desired_portable_experiences.size()
		desired_portable_experiences.erase(pid)
		assert(desired_portable_experiences.size() == n - 1)

		if pid_to_world.get(pid) != null:
			pid = pid_to_world[pid]
			desired_portable_experiences.erase(pid)

		update_portable_experiences()


func _on_scene_spawned(scene_id: int, entity_id: String):
	if entity_id_to_pid.get(entity_id) == null:
		return

	var pid: String = entity_id_to_pid.get(entity_id)
	if desired_portable_experiences.find(pid) == -1:
		return

	if pid_to_world.get(pid) != null:
		pid = pid_to_world[pid]

	self.announce_spawned(pid, true, "", scene_id)
