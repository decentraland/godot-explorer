extends DclPortableExperienceController
class_name PortableExperienceController

var desired_portable_experiences: Array[String] = []
var killing_ids: Array[String] = []
var spawning_ids: Array[String] = []


func _ready():
	Global.scene_runner.scene_killed.connect(self._on_scene_killed)
	Global.scene_runner.scene_spawned.connect(self._on_scene_spawned)


func _process(delta):
	var to_spawn := self.consume_requested_spawn()
	if not to_spawn.is_empty():
		spawn_many_portables(to_spawn)

	var to_kill := self.consume_requested_kill()
	if not to_kill.is_empty():
		kill_many_portables(to_kill)


func spawn_many_portables(pids: Array[String]) -> void:
	for pid in pids:
		spawn_portable_experience(pid)


func kill_many_portables(pids: Array[String]) -> void:
	for pid in pids:
		kill_portable_experience(pid)


func spawn_portable_experience(pid: String) -> void:
	if Realm.is_dcl_ens(pid):
		var world_realm = Realm.new()
		world_realm.set_realm(pid)
		await world_realm.realm_changed

		# TODO: complete this path
		pid = "new_pid"

	desired_portable_experiences.push_back(pid)


func kill_portable_experience(pid: String) -> void:
	desired_portable_experiences.erase(pid)


func _on_scene_killed(scene_id: int, _entity_id: String):
	prints("_on_scene_killed", scene_id)
	self.announce_killed_by_scene_id(scene_id)


func _on_scene_spawned(scene_id: int, entity_id: String):
	if not desired_portable_experiences.find(entity_id):
		prints("_on_scene_spawned not found ", entity_id)
		return

	prints("_on_scene_spawned found", entity_id)
	self.announce_spawned(entity_id, true, "", scene_id)
