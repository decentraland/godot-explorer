extends SceneTree

# Headless mirror of player.gd's relocation step-up (PhysX three-phase) +
# arming + support re-arm. Cases modeled on kuruk.dcl.eth's matrix, incl. a
# seam-lip step (stacked colliders).

const SPEED := 3.0
const STEP_MIN_RISE := 0.02
const STEP_TALL_RISE := 0.25
const STEP_PENDING_RISE := 0.05
const STEP_MAX_HEIGHT := 0.4375
const STEP_MAX_DIP := 0.1
const WALKABLE_NORMAL_Y := 0.695
const SLOPE_WALK_NORMAL_Y := 0.99
const CAPSULE_RADIUS := 0.3
const CAPSULE_CENTER_Y := 0.8

var _body: CharacterBody3D
var _capsule: CapsuleShape3D
var _step_shape: CapsuleShape3D
var _armed := true
var _pending := false
var _frame := 0
var world: Node3D

# per-case max height
var _max_y := -99.0
var _results: Array = []
var _label := ""
var _bails := {}


func _make_box(pos: Vector3, size: Vector3) -> StaticBody3D:
	var b := StaticBody3D.new()
	b.collision_layer = 2
	b.position = pos
	var col := CollisionShape3D.new()
	var box := BoxShape3D.new()
	box.size = size
	col.shape = box
	b.add_child(col)
	world.add_child(b)
	return b


func _make_ramp(z: float, deg: float, length: float) -> void:
	var a := deg_to_rad(deg)
	# low end surface touches ground (y=0); rotX(+deg) raises the -z end
	var center_y := sin(a) * length * 0.5 - 0.05
	var s := StaticBody3D.new()
	s.collision_layer = 2
	s.position = Vector3(0, center_y, z)
	var col := CollisionShape3D.new()
	var box := BoxShape3D.new()
	box.size = Vector3(2.0, 0.1, length)
	col.shape = box
	col.rotation_degrees.x = deg
	s.add_child(col)
	world.add_child(s)


func _initialize() -> void:
	print("ENGINE=", ProjectSettings.get_setting("physics/3d/physics_engine", "DEFAULT"))
	world = Node3D.new()
	world.name = "m"
	world.process_mode = Node.PROCESS_MODE_ALWAYS
	get_root().add_child.call_deferred(world)
	await process_frame
	await physics_frame

	# ground: top at y=0
	_make_box(Vector3(0, -1, -60), Vector3(30, 2, 140))

	# A: plain step 0.30 at z=-5
	_make_box(Vector3(0, 0.15, -5), Vector3(2, 0.3, 1))
	# ladder: separate blocks 0.40/0.425/0.435/0.44/0.45 at z=-10/-15/-20/-25/-30
	# (kuruk's real tops: labels 0.42/0.43 measure ~+5mm from seam lips)
	var hs := [0.40, 0.425, 0.435, 0.44, 0.45]
	for i in hs.size():
		_make_box(Vector3(0, hs[i] * 0.5, -10 - i * 5), Vector3(2, hs[i], 1))
	# staircase 4x0.15 at z=-40 (treads 0.3 deep)
	for i in 4:
		_make_box(Vector3(0, 0.15 * (i + 1) * 0.5, -40 - i * 0.3), Vector3(2, 0.15 * (i + 1), 0.3))
	# bevel 60deg curb at z=-47 (top ~0.25)
	var bev := _make_box(Vector3(0, 0.125, -47), Vector3(2, 0.183, 0.183))
	bev.rotation_degrees.x = 60.0
	# seam-lip step at z=-55: platform 0.42 + thin plate lip topping at 0.425
	_make_box(Vector3(0, 0.21, -55), Vector3(2, 0.42, 1))
	_make_box(Vector3(0, 0.42 + 0.0025, -54.7), Vector3(2, 0.005, 0.4))
	# ramps at z=-62/-70/-78/-86/-94 (45/50/55/60/65)
	_make_ramp(-62, 45.0, 3.0)
	_make_ramp(-70, 50.0, 3.0)
	_make_ramp(-78, 55.0, 3.0)
	_make_ramp(-86, 60.0, 3.0)
	_make_ramp(-94, 65.0, 3.0)
	_make_ramp(-107, 46.0, 3.0)
	# GAPSTEP: stand on a 0.42 platform, a block whose top is +0.437 across a
	# 0.2 gap (kuruk's ladder blocks are separated — rays can land in the gap)
	_make_box(Vector3(0, 0.21, -100), Vector3(2, 0.42, 1))
	_make_box(Vector3(0, 0.4285, -101.2), Vector3(2, 0.857, 1))

	# player body
	_body = CharacterBody3D.new()
	_body.collision_layer = 0
	_body.collision_mask = 2
	_body.floor_max_angle = deg_to_rad(45.99)
	_body.motion_mode = CharacterBody3D.MOTION_MODE_GROUNDED
	_body.safe_margin = 0.01
	_capsule = CapsuleShape3D.new()
	_capsule.radius = CAPSULE_RADIUS
	_capsule.height = 1.6
	_capsule.margin = 0.08
	var col := CollisionShape3D.new()
	col.shape = _capsule
	col.position = Vector3(0, CAPSULE_CENTER_Y, 0)
	_body.add_child(col)
	_body.global_position = Vector3(0, 0.0, 0)
	world.add_child(_body)
	_step_shape = _capsule.duplicate()
	_step_shape.margin = 0.0


func _bail(why: String) -> void:
	_bails[why] = _bails.get(why, 0) + 1


func _has_walkable_support() -> bool:
	if _body.is_on_floor():
		return true
	if _body.global_position.y <= 0.005:
		return true  # clamp-held realm floor (live: no collider contact at y=0)
	var space := _body.get_world_3d().direct_space_state
	var rq := PhysicsRayQueryParameters3D.new()
	rq.from = _body.global_position + Vector3(0.0, 0.05, 0.0)
	rq.to = _body.global_position + Vector3(0.0, -0.1, 0.0)
	rq.collision_mask = 2
	var hit := space.intersect_ray(rq)
	return not hit.is_empty() and hit.normal.y >= WALKABLE_NORMAL_Y


func _try_predictive(intent: Vector3, dt: float) -> void:
	var horiz := Vector3(intent.x, 0.0, intent.z)
	if horiz.length_squared() < 0.25:
		return
	if _body.is_on_floor() and _body.get_floor_normal().y < SLOPE_WALK_NORMAL_Y:
		return
	var space := _body.get_world_3d().direct_space_state
	var q := PhysicsShapeQueryParameters3D.new()
	q.shape = _step_shape
	q.collision_mask = 2
	q.transform = Transform3D(
		Basis.IDENTITY, _body.global_position + Vector3(0.0, CAPSULE_CENTER_Y, 0.0)
	)
	q.motion = horiz.normalized() * (horiz.length() * dt + 0.05)
	var contact: PackedFloat32Array = space.cast_motion(q)
	if contact[0] >= 1.0:
		return
	_step_up(intent)


func _try_step_up(intent: Vector3, moved_xz: float) -> void:
	var expected := Vector3(intent.x, 0, intent.z).length() * (1.0 / 60.0)
	var blocked := expected > 0.008 and moved_xz < expected * 0.3
	if not _body.is_on_wall() and not blocked:
		return
	_step_up(intent)


func _step_up(intent: Vector3) -> void:
	if not _armed:
		_bail("armed")
		return
	var horiz := Vector3(intent.x, 0.0, intent.z)
	if horiz.length_squared() < 0.25:
		return
	var space := _body.get_world_3d().direct_space_state
	var dir := horiz.normalized()
	var origin := _body.global_position + Vector3(0.0, CAPSULE_CENTER_Y, 0.0)
	var q := PhysicsShapeQueryParameters3D.new()
	q.shape = _step_shape
	q.collision_mask = 2
	# a) face distance: margin-less clone travel + radius (needed to PLACE the
	#    down phase past the face — the landing height itself is never
	#    thresholded; the lift cap below enforces the band)
	q.transform = Transform3D(Basis.IDENTITY, origin)
	q.motion = dir * 0.5
	var low: PackedFloat32Array = space.cast_motion(q)
	if low[1] >= 1.0:
		return  # no face within cast range — nothing to step onto
	var d_face := low[1] * 0.5 + CAPSULE_RADIUS
	# b) up: how far the capsule can be lifted (doubles as the headroom check)
	q.motion = Vector3(0.0, STEP_MAX_HEIGHT, 0.0)
	var up: PackedFloat32Array = space.cast_motion(q)
	var lift := up[0] * STEP_MAX_HEIGHT
	if lift < STEP_MIN_RISE:
		_bail("ceiling")
		return  # ceiling
	# c) forward at lifted height until the pole sits just past the face:
	#    a riser taller than the lift blocks this => wall
	var over_face := d_face + 0.005
	q.transform = Transform3D(Basis.IDENTITY, origin + Vector3(0.0, lift, 0.0))
	q.motion = dir * over_face
	var fw: PackedFloat32Array = space.cast_motion(q)
	if fw[0] < 1.0:
		_bail("wall")
		return  # cannot place the capsule over the edge at lifted height — wall
	# d) down with the pole directly over the edge: no hemisphere graze, the
	#    top reads exact; the capsule footprint catches thin lips. Void past
	#    the band => a gap, not a step (the capsule steps UP only).
	var landing := origin + Vector3(0.0, lift, 0.0) + dir * over_face
	q.transform = Transform3D(Basis.IDENTITY, landing)
	q.motion = Vector3(0.0, -(lift + 0.02), 0.0)
	var dn: PackedFloat32Array = space.cast_motion(q)
	if dn[0] >= 1.0:
		_bail("gap")
		return  # gap
	if dn[0] < 0.005:
		# The down phase starts already touching: the capsule is squeezed
		# against the face/edge at lifted height — a wall, not a step. (Reading
		# it as a landing reports floor_y = lifted height and teleports up.)
		_bail("squeezed")
		return
	var floor_y: float = landing.y - dn[0] * (lift + 0.02) - _step_shape.height * 0.5
	if floor_y < _body.global_position.y + STEP_MIN_RISE:
		_bail("flat")
		return  # flat or lower — nothing to step onto
	# Never commit an overlapping relocation (CCT invariant): diagonal corner
	# approaches can thread the lifted forward cast past the block's edge and
	# start the down phase with the pole inside the top.
	q.motion = Vector3.ZERO
	q.transform = Transform3D(
		Basis.IDENTITY, Vector3(landing.x, floor_y + CAPSULE_CENTER_Y, landing.z)
	)
	if not space.intersect_shape(q, 1).is_empty():
		_bail("overlap")
		return
	# The highest walkable surface under the landing footprint decides: probe
	# at the edge, mid-tread, and a capsule-radius past (diagonal corner
	# approaches park the pole beside the block). The body cast can graze an
	# edge and misread it; the rays read exact tops. On a triple miss
	# (trimesh tri edges) keep the body-cast height.
	var prq := PhysicsRayQueryParameters3D.new()
	prq.collision_mask = 2
	var best_y := -INF
	for dist in [d_face + 0.005, d_face + 0.15, d_face + 0.3]:
		var px: float = _body.global_position.x + dir.x * dist
		var pz: float = _body.global_position.z + dir.z * dist
		prq.from = Vector3(px, _body.global_position.y + lift + 0.1, pz)
		prq.to = Vector3(px, _body.global_position.y - 0.05, pz)
		var phit := space.intersect_ray(prq)
		if not phit.is_empty() and phit.position.y > best_y:
			best_y = phit.position.y
	if best_y == -INF:
		# All probe points missed (a gap between blocks, or the landing slid
		# off the obstacle). Keeping the body-cast height here is how walls get
		# climbed: its graze contact always reads just under the band.
		_bail("miss")
		return
	if best_y > -INF:
		floor_y = best_y
		# The rise is measured from the SUPPORT under the feet, not the pole:
		# resting on an edge with the bottom hemisphere lifts the pole ~1cm,
		# which would smuggle a 0.437 obstacle under the 0.43 band.
		var support_y := _body.global_position.y
		var srq := PhysicsRayQueryParameters3D.new()
		srq.collision_mask = 2
		srq.from = _body.global_position + Vector3(0.0, 0.05, 0.0)
		srq.to = _body.global_position + Vector3(0.0, -0.15, 0.0)
		var shit := space.intersect_ray(srq)
		if not shit.is_empty():
			support_y = shit.position.y
		if floor_y > support_y + STEP_MAX_HEIGHT:
			_bail("tall")
			return  # the corner/top is above the band after all
		if floor_y < _body.global_position.y + STEP_MIN_RISE:
			_bail("flat-ray")
			return  # pole over void/lower ground — the cast contact was a graze
	# The capsule's center rests ~a radius past the face: that surface must be
	# walkable. A beveled curb is past its slope there (flat), a staircase
	# tread is flat, a continuous ramp still reads its slope — that's what
	# stops the climb/slide loop. A ray miss accepts (trimesh tri edges).
	var nrq := PhysicsRayQueryParameters3D.new()
	nrq.collision_mask = 2
	var nx: float = _body.global_position.x + dir.x * (d_face + 0.25)
	var nz: float = _body.global_position.z + dir.z * (d_face + 0.25)
	nrq.from = Vector3(nx, _body.global_position.y + lift + 0.1, nz)
	nrq.to = Vector3(nx, _body.global_position.y - 0.05, nz)
	var nhit := space.intersect_ray(nrq)
	if not nhit.is_empty() and nhit.normal.y < WALKABLE_NORMAL_Y:
		_bail("ramp")
		return  # the capsule would rest on an un-walkable slope — a ramp
	var rise := floor_y - _body.global_position.y
	if rise > STEP_TALL_RISE or (rise >= STEP_PENDING_RISE and _pending):
		_armed = false
	elif rise >= STEP_PENDING_RISE:
		_pending = true
	_body.floor_snap_length = 0.0
	_body.global_position.y = floor_y + 0.001


func _drive(intent: Vector3, dt: float) -> void:
	_body.velocity.y -= 10.0 * dt
	_body.floor_snap_length = 0.45
	_body.velocity.x = intent.x
	_body.velocity.z = intent.z
	var before := _body.global_position
	_try_predictive(intent, dt)
	var vy_before := _body.velocity.y
	var y_before := _body.global_position.y
	_body.move_and_slide()
	if _body.global_position.y > y_before + 0.0005 and vy_before <= 0.0:
		# The move itself gained height without upward velocity: a slope slide
		# carried us. Allowed on walkable contacts only (PhysX slope limiter).
		var walkable := false
		var any := false
		for i in _body.get_slide_collision_count():
			any = true
			if _body.get_slide_collision(i).get_normal().y >= WALKABLE_NORMAL_Y:
				walkable = true
		if any and not walkable:
			_body.global_position.y = before.y
			_body.velocity.y = minf(_body.velocity.y, 0.0)
	var moved := Vector2(
		_body.global_position.x - before.x, _body.global_position.z - before.z
	).length()
	_try_step_up(intent, moved)
	if (not _armed or _pending) and _has_walkable_support():
		_armed = true
		_pending = false
	_body.global_position.y = max(_body.global_position.y, 0)


# case schedule: [start_frame, teleport_z, label] — 100 frames = 5m at 3m/s
var _cases := [
	[1, -3.0, "A_step0.30"],
	[101, -8.5, "L40"],
	[201, -13.5, "L42"],
	[301, -18.5, "L43"],
	[401, -23.5, "L44"],
	[501, -28.5, "L45"],
	[601, -37.5, "C_stairs"],
	[701, -45.0, "D_bevel"],
	[801, -53.0, "SEAM_lip"],
	[901, -58.5, "R45"],
	[1001, -66.5, "R50"],
	[1101, -74.5, "R55"],
	[1201, -82.5, "R60"],
	[1301, -90.5, "R65"],
	[1401, -22.8, "DIAG44"],
	[1501, -12.8, "DIAG42"],
	[1601, -99.7, "GAPSTEP"],
	[1701, -104.5, "R46"],
]
var _case_end := 1801


func _physics_process(_delta: float) -> bool:
	if _body == null:
		return false
	_frame += 1
	var dt := 1.0 / 60.0
	if _frame >= _case_end:
		_finish()
		return false
	for c in _cases:
		if _frame == c[0]:
			_report()
			_label = c[2]
			_max_y = -99.0
			_body.global_position = Vector3(
				0.4 if _label.begins_with("DIAG") else 0.0,
				0.42 if _label == "GAPSTEP" else 0.0,
				c[1]
			)
			_body.velocity = Vector3.ZERO
			_armed = true
			_pending = false
			break
	_max_y = maxf(_max_y, _body.global_position.y)
	if _label.begins_with("DIAG"):
		_drive(Vector3(-0.35, 0, -1).normalized() * SPEED, dt)
	else:
		_drive(Vector3(0, 0, -SPEED), dt)
	return false


func _finish() -> void:
	_report()
	for r in _results:
		print(r)
	quit(0)


func _report() -> void:
	if _label == "":
		return
	var z := _body.global_position.z
	_results.append(
		"%s max_y=%.3f end_z=%.2f end_y=%.3f bails=%s"
		% [_label, _max_y, z, _body.global_position.y, _bails]
	)
	_bails = {}
