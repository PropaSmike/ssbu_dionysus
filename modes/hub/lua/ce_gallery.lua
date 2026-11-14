-- A gallery: one [[gallery]] from a pack's menu.toml, opened from the modes
-- page. It runs on the Mii Fighters list screen, which already keeps a 3D
-- world behind its menus. Opened any other way the screen stays as it was.
--
-- Two looks share this script: the page's own panels, and a grid skin (a
-- second layout with the panes skin_page and skin_view, see docs/wiki/MENUS.md
-- "Skins"). A pane the layout lacks still answers get_pane, so the skin's
-- sizes are fixed numbers below, not read from the layout.

UiScriptPlayer.require("menu/chara_make/make_mii_select_actor")

local call = UiScriptPlayer.engine_call
local text = UiScriptPlayer.engine_text
local item_count = call("engine.gallery_count")

if type(item_count) ~= "number" or item_count < 0 then
  return
end

local BUTTON_A = 0x1
local BUTTON_B = 0x2
local BUTTON_X = 0x4
local BUTTON_Y = 0x8
local BUTTON_L = 0x40
local BUTTON_R = 0x80
local BUTTON_ZL = 0x100
local BUTTON_ZR = 0x200
local DPAD_LEFT = 0x1000
local DPAD_UP = 0x2000
local DPAD_RIGHT = 0x4000
local DPAD_DOWN = 0x8000
local STICK_LEFT = 0x10000
local STICK_UP = 0x20000
local STICK_RIGHT = 0x40000
local STICK_DOWN = 0x80000
local MOVE_UP = DPAD_UP | STICK_UP
local MOVE_DOWN = DPAD_DOWN | STICK_DOWN
local MOVE_LEFT = DPAD_LEFT | STICK_LEFT
local MOVE_RIGHT = DPAD_RIGHT | STICK_RIGHT
local TURN_LEFT = 0x100000
local TILT_UP = 0x200000
local TURN_RIGHT = 0x400000
local TILT_DOWN = 0x800000

local ROWS = 9
local LAYOUT_INDEX = 3
local SKIN_SEARCH = 8
local PRIORITY = 0x1100
local SKIN_PRIORITY = 0x1000
local SLOT = 0
local STATE_READY = 1
local STATE_SHOWN = 2
local PLACEHOLDER = "ui_chara_mario"
local TURN_STEP = 3.0
local TILT_STEP = 1.5
local PAN_STEP = 0.2
local ZOOM_RATE = 0.012
local ZOOM_START = 1.36
local ZOOM_MIN = 0.4
local ZOOM_MAX = 3.0
local GRID_ZOOM = 1.9
local GRID_CELLS = 12
local GRID_COLUMNS = 3
local DESC_LINES = 9
local DESC_WRAP = 60
local SOURCE_ROWS = 2
local HELP_LABELS = { "Rotate", "Slide", "Zoom in", "Zoom out", "Switch view", "Back", "Help: ON/OFF" }
local ANGLE_ZOOM = 1.0
local ANGLE_SHIFT = 0.0
local CAMERA_X = 7.1
local CAMERA_Y = 10.0
local CAMERA_RAISE = -2.63
local CAMERA_Z = 136.7
local CAMERA_FOV = 0.17
local REPEAT_DELAY = 20
local REPEAT_EVERY = 6
local WRAP_COLUMNS = 74
local TEXT_LINES = 5
local HINT = "Left stick: select, scroll text    L / R: page    Right stick: turn    ZL / ZR: zoom    Y: reset    B: back"

local ui = {}
local view = {
  selected = 1, top = 1, wanted = 1, loaded = 0,
  yaw = 0.0, tilt = 0.0, scale = ZOOM_START, pan_x = 0.0, pan_y = 0.0,
  placeholder = false, sent = false, shown = false,
  previous = 0, frames = 0, held_for = 0, held_dir = nil, text_held = 0, lines = {}, line = 1, page = 1, pages = 1,
  mode = "view", help = true, angle = false, done = false,
}

local function log(...)
  call("engine.log", "gallery:", ...)
end

local SOUND_ID = {
  cursor = function() return UI_SE_ID_CURSOR end,
  decide = function() return UI_SE_ID_DECIDE end,
  cancel = function() return UI_SE_ID_CANCEL end,
  page = function() return UI_SE_ID_PAGE_CHANGE end,
}
local SOUND_LABEL = {
  cursor = "se_system_cursor_l",
  decide = "se_system_fixed_s",
  cancel = "se_system_cancel",
  page = "se_system_cursor_l",
}

local sound_logged = {}

local function sound(kind)
  local ok, id = pcall(SOUND_ID[kind])
  local played = ok and type(id) == "number" and pcall(UiSoundManager.play_se, id)
  if not played then
    played = pcall(function()
      UiSoundManager:play_se_label(SOUND_LABEL[kind])
    end)
  end
  if sound_logged[kind] == nil then
    sound_logged[kind] = true
    log("sound", kind, tostring(id), played)
  end
end

local function item_text(name, index)
  local value = text(name, index - 1)
  if type(value) ~= "string" then
    return ""
  end
  return value
end

local function wrapped(source, columns)
  local lines = {}
  for paragraph in (source .. "\n"):gmatch("(.-)\n") do
    local line = ""
    for word in paragraph:gmatch("%S+") do
      if #line == 0 then
        line = word
      elseif #line + 1 + #word <= columns then
        line = line .. " " .. word
      else
        lines[#lines + 1] = line
        line = word
      end
    end
    lines[#lines + 1] = line
  end
  return lines
end

local function pane_of(view_root, name)
  local ok, pane = pcall(function()
    return view_root:get_pane(name)
  end)
  if ok then
    return pane
  end
  return nil
end

local function set_text(pane, value)
  if pane ~= nil then
    pane:set_text_string(value)
  end
end

local function set_visible(pane, value)
  if pane ~= nil then
    pane:set_visible(value)
  end
end

local NAME_FIT = { 11, 18 }
local LABEL_FIT = { 14, 19 }

local function sized(panes, value, fit)
  local size = 1
  if #value > fit[2] then
    size = 3
  elseif #value > fit[1] then
    size = 2
  end
  for index = 1, 3 do
    set_text(panes[index], index == size and value or "")
  end
end

local function draw_text()
  local lines = view.lines
  local last = math.min(view.line + TEXT_LINES - 1, #lines)
  local shown = table.concat(lines, "\n", view.line, last)
  if last < #lines then
    shown = shown .. " ..."
  end
  ui.body:set_text_string(shown)
end

local function apply_camera()
  local scale = view.scale
  local shift = CAMERA_X
  if view.mode == "grid" then
    shift = 0.0
  elseif view.angle then
    shift = ANGLE_SHIFT
  end
  local x = shift * scale + view.pan_x
  local y = CAMERA_Y + CAMERA_RAISE * scale + view.pan_y
  UiFighterManager.set_camera_pos(x, y, CAMERA_Z * scale, x, y, 0.0, CAMERA_FOV)
end

local function apply_rotation()
  UiFighterManager.set_fighter_rotation_by_degree(FIGHTER_ENTRY_ID_SELF, view.tilt, view.yaw, 0.0)
end

local function reset_view()
  view.yaw = 0.0
  view.tilt = 0.0
  view.scale = view.angle and ANGLE_ZOOM or ZOOM_START
  view.pan_x = 0.0
  view.pan_y = 0.0
end

local function build_groups()
  local groups = {}
  for index = 1, item_count do
    local name = item_text("engine.gallery_group", index)
    local last = groups[#groups]
    if last ~= nil and last.name == name then
      last.last = index
    else
      groups[#groups + 1] = { name = name, first = index, last = index }
    end
  end
  return groups
end

local function build_pages(groups, cells)
  local pages = {}
  local page_of = {}
  for _, group in ipairs(groups) do
    local count = math.ceil((group.last - group.first + 1) / cells)
    local first = group.first
    local number = 1
    while first <= group.last do
      local last = math.min(group.last, first + cells - 1)
      pages[#pages + 1] = { group = group, first = first, last = last, number = number, count = count }
      for index = first, last do
        page_of[index] = #pages
      end
      first = last + 1
      number = number + 1
    end
  end
  return pages, page_of
end

local function thumb(index)
  local skin = ui.skin
  local found = skin.thumbs[index]
  if found == nil then
    found = pane_of(skin.layout, "skin_thumb_" .. index) or false
    skin.thumbs[index] = found
  end
  return found or nil
end

local function draw_grid()
  local skin = ui.skin
  local page = skin.pages[skin.page_of[view.selected]]
  if skin.shown_page ~= page then
    if skin.shown_page ~= nil then
      for index = skin.shown_page.first, skin.shown_page.last do
        set_visible(thumb(index), false)
      end
    end
    for index = page.first, page.last do
      set_visible(thumb(index), true)
    end
    skin.shown_page = page
  end
  set_text(skin.grid_group, page.group.name)
  set_text(skin.grid_foot, page.group.name)
  set_text(skin.grid_page, string.format("(%d/%d)", page.number, page.count))
  set_text(skin.grid_total, string.format("(%d)", item_count))
  for slot = 1, #skin.cells do
    local index = page.first + slot - 1
    local present = index <= page.last
    sized(skin.cells[slot].labels, present and item_text("engine.gallery_name", index) or "", LABEL_FIT)
    set_visible(skin.cells[slot].cursor, present and index == view.selected)
  end
end

local function draw_description()
  local skin = ui.skin
  local first = (view.page - 1) * skin.lines + 1
  local last = math.min(first + skin.lines - 1, #view.lines)
  set_text(skin.desc, table.concat(view.lines, "\n", first, last))
  set_text(skin.desc_page, string.format("(%d/%d)", view.page, view.pages))
end

local function show_parts()
  local skin = ui.skin
  set_visible(skin.info, not view.angle)
  set_visible(skin.angle, view.angle)
  set_visible(skin.help, view.angle and view.help)
end

local function draw_view()
  local skin = ui.skin
  sized(skin.names, item_text("engine.gallery_name", view.selected), NAME_FIT)
  view.lines = wrapped(item_text("engine.gallery_text", view.selected), skin.wrap)
  view.pages = math.max(1, math.ceil(#view.lines / skin.lines))
  view.page = 1
  draw_description()
  local sources = wrapped(item_text("engine.gallery_sources", view.selected), 1000)
  local present = {}
  for _, line in ipairs(sources) do
    if #line > 0 then
      present[#present + 1] = line
    end
  end
  for slot = 1, #skin.sources do
    local entry = present[slot]
    local label, tag = nil, nil
    if entry ~= nil then
      label, tag = entry:match("^(.-)%s*|([%w_]+)$")
      if label == nil then
        label = entry
      end
    end
    local row = skin.sources[slot]
    set_visible(row.row, entry ~= nil)
    set_text(row.label, label or "")
    set_visible(row.icon, false)
    row.icon = nil
    if tag ~= nil then
      local icon = pane_of(skin.layout, "skin_source_icon_" .. (slot - 1) .. "_" .. tag)
      set_visible(icon, true)
      row.icon = icon
    end
  end
  show_parts()
end

local function draw()
  set_text(ui.count, string.format("%d / %d", view.selected, item_count))
  if ui.v2 then
    if view.mode == "grid" then
      draw_grid()
    else
      draw_view()
    end
    return
  end
  ui.name:set_text_string(item_text("engine.gallery_name", view.selected))
  view.lines = wrapped(item_text("engine.gallery_text", view.selected), WRAP_COLUMNS)
  view.line = 1
  draw_text()
  for row = 1, ROWS do
    local index = view.top + row - 1
    local present = index <= item_count
    ui.rows[row].pane:set_visible(present)
    if present then
      ui.rows[row].label:set_text_string(item_text("engine.gallery_name", index))
      ui.rows[row].on:set_visible(index == view.selected)
    end
  end
end

local function select(index, kind)
  if index < 1 then
    index = item_count
  elseif index > item_count then
    index = 1
  end
  view.selected = index
  if index < view.top then
    view.top = index
  elseif index > view.top + ROWS - 1 then
    view.top = index - ROWS + 1
  end
  if ui.v2 ~= true or view.mode == "view" then
    if view.wanted ~= index then
      reset_view()
    end
    view.wanted = index
  end
  sound(kind or "cursor")
  draw()
end

local function jump(by)
  local index = view.selected + by
  if index < 1 then
    index = 1
  elseif index > item_count then
    index = item_count
  end
  select(index, "page")
end

local function find_skin()
  for index = LAYOUT_INDEX + 1, LAYOUT_INDEX + SKIN_SEARCH do
    local found = nil
    local ok = pcall(function()
      local root = LayoutRootList[index]
      root:activate_layout(SKIN_PRIORITY)
      local skin_layout = root:get_root_view()
      found = { root = root, layout = skin_layout, page = skin_layout:get_pane("skin_page") }
    end)
    if ok and found ~= nil then
      return found
    end
  end
  return nil
end

local function setup_grid_skin(skin)
  local layout = skin.layout
  skin.grid = pane_of(layout, "skin_grid")
  skin.view = pane_of(layout, "skin_view")
  skin.names = { pane_of(layout, "skin_name"), pane_of(layout, "skin_mid_name"), pane_of(layout, "skin_tiny_name") }
  skin.desc = pane_of(layout, "skin_desc")
  skin.desc_page = pane_of(layout, "skin_desc_page")
  skin.help = pane_of(layout, "skin_help")
  skin.info = pane_of(layout, "skin_info")
  skin.angle = pane_of(layout, "skin_angle")
  skin.grid_group = pane_of(layout, "skin_grid_group")
  skin.grid_page = pane_of(layout, "skin_grid_page")
  skin.grid_total = pane_of(layout, "skin_grid_total")
  skin.grid_foot = pane_of(layout, "skin_grid_foot")
  skin.columns = GRID_COLUMNS
  skin.lines = DESC_LINES
  skin.wrap = DESC_WRAP
  skin.thumbs = {}
  skin.cells = {}
  for slot = 0, GRID_CELLS - 1 do
    skin.cells[slot + 1] = {
      cursor = pane_of(layout, "skin_cell_cursor_" .. slot),
      labels = {
        pane_of(layout, "skin_cell_label_" .. slot),
        pane_of(layout, "skin_cell_small_" .. slot),
        pane_of(layout, "skin_cell_tiny_" .. slot),
      },
    }
  end
  skin.sources = {}
  for slot = 0, SOURCE_ROWS - 1 do
    skin.sources[slot + 1] = { row = pane_of(layout, "skin_source_" .. slot), label = pane_of(layout, "skin_source_text_" .. slot), icon = nil }
  end
  for slot, label in ipairs(HELP_LABELS) do
    set_text(pane_of(layout, "skin_help_text_" .. (slot - 1)), label)
  end
  skin.groups = build_groups()
  skin.pages, skin.page_of = build_pages(skin.groups, math.max(1, #skin.cells))
end

local function set_mode(mode)
  view.mode = mode
  local skin = ui.skin
  set_visible(skin.grid, mode == "grid")
  set_visible(skin.view, mode == "view")
  if mode == "view" then
    view.wanted = view.selected
    view.angle = false
    reset_view()
  else
    view.scale = GRID_ZOOM
    view.pan_x = 0.0
    view.pan_y = 0.0
  end
  if view.shown then
    apply_camera()
    apply_rotation()
  end
  draw()
end

local function setup()
  local root = LayoutRootList[LAYOUT_INDEX]
  root:activate_layout(PRIORITY)
  local layout = root:get_root_view()
  ui.root = root
  ui.page = layout:get_pane("page")
  ui.count = layout:get_pane("count")
  ui.name = layout:get_pane("item_name")
  ui.body = layout:get_pane("item_text")
  ui.rows = {}
  for row = 1, ROWS do
    local prefix = "row_" .. (row - 1)
    ui.rows[row] = {
      pane = layout:get_pane(prefix),
      base = layout:get_pane(prefix .. "_base"),
      on = layout:get_pane(prefix .. "_on"),
      label = layout:get_pane(prefix .. "_label"),
    }
  end
  layout:get_pane("title"):set_text_string(text("engine.gallery_title"))
  layout:get_pane("hint"):set_text_string(HINT)
  ui.skin = find_skin()
  if ui.skin ~= nil then
    ui.v2 = true
    setup_grid_skin(ui.skin)
    ui.skin.page:set_visible(true)
    ui.page:set_visible(false)
    return
  end
  ui.page:set_visible(true)
end

local function moved(held, mask)
  return held & mask ~= 0
end

local function read_movement(held, pressed)
  if pressed & MOVE_UP ~= 0 then
    view.held_for = 0
    select(view.selected - 1)
  elseif pressed & MOVE_DOWN ~= 0 then
    view.held_for = 0
    select(view.selected + 1)
  elseif moved(held, MOVE_UP) or moved(held, MOVE_DOWN) then
    view.held_for = view.held_for + 1
    if view.held_for >= REPEAT_DELAY and (view.held_for - REPEAT_DELAY) % REPEAT_EVERY == 0 then
      if moved(held, MOVE_UP) then
        select(view.selected - 1)
      else
        select(view.selected + 1)
      end
    end
  else
    view.held_for = 0
  end
  if pressed & BUTTON_L ~= 0 then
    jump(-ROWS)
  elseif pressed & BUTTON_R ~= 0 then
    jump(ROWS)
  end
end

local function read_text(held, pressed)
  local step = 0
  if moved(held, MOVE_LEFT) then
    step = -1
  elseif moved(held, MOVE_RIGHT) then
    step = 1
  end
  if step == 0 then
    view.text_held = 0
    return
  end
  if pressed & (MOVE_LEFT | MOVE_RIGHT) ~= 0 then
    view.text_held = 0
  else
    view.text_held = view.text_held + 1
    if view.text_held < REPEAT_DELAY or (view.text_held - REPEAT_DELAY) % REPEAT_EVERY ~= 0 then
      return
    end
  end
  local line = math.max(1, math.min(math.max(1, #view.lines - TEXT_LINES + 1), view.line + step))
  if line ~= view.line then
    view.line = line
    draw_text()
  end
end

local function read_view(held, pressed)
  local turned = false
  if moved(held, TURN_LEFT) then
    view.yaw = view.yaw - TURN_STEP
    turned = true
  elseif moved(held, TURN_RIGHT) then
    view.yaw = view.yaw + TURN_STEP
    turned = true
  end
  if moved(held, TILT_UP) then
    view.tilt = view.tilt - TILT_STEP
    turned = true
  elseif moved(held, TILT_DOWN) then
    view.tilt = view.tilt + TILT_STEP
    turned = true
  end
  if pressed & BUTTON_Y ~= 0 then
    sound("cursor")
    reset_view()
    turned = true
    apply_camera()
  end
  if turned then
    apply_rotation()
  end
  local zoom = 0.0
  if moved(held, BUTTON_ZR) then
    zoom = -ZOOM_RATE
  elseif moved(held, BUTTON_ZL) then
    zoom = ZOOM_RATE
  end
  if zoom ~= 0.0 then
    view.scale = math.max(ZOOM_MIN, math.min(ZOOM_MAX, view.scale + zoom))
    apply_camera()
  end
end

local function read_inspect(held, pressed)
  local turned = false
  if moved(held, STICK_LEFT) then
    view.yaw = view.yaw - TURN_STEP
    turned = true
  elseif moved(held, STICK_RIGHT) then
    view.yaw = view.yaw + TURN_STEP
    turned = true
  end
  if moved(held, STICK_UP) then
    view.tilt = view.tilt - TILT_STEP
    turned = true
  elseif moved(held, STICK_DOWN) then
    view.tilt = view.tilt + TILT_STEP
    turned = true
  end
  if turned then
    apply_rotation()
  end
  local moved_camera = false
  if moved(held, TURN_LEFT) then
    view.pan_x = view.pan_x + PAN_STEP
    moved_camera = true
  elseif moved(held, TURN_RIGHT) then
    view.pan_x = view.pan_x - PAN_STEP
    moved_camera = true
  end
  if moved(held, TILT_UP) then
    view.pan_y = view.pan_y - PAN_STEP
    moved_camera = true
  elseif moved(held, TILT_DOWN) then
    view.pan_y = view.pan_y + PAN_STEP
    moved_camera = true
  end
  local zoom = 0.0
  if moved(held, BUTTON_X) then
    zoom = -ZOOM_RATE
  elseif moved(held, BUTTON_Y) then
    zoom = ZOOM_RATE
  end
  if zoom ~= 0.0 then
    view.scale = math.max(ZOOM_MIN, math.min(ZOOM_MAX, view.scale + zoom))
    moved_camera = true
  end
  if pressed & BUTTON_ZL ~= 0 then
    sound("cursor")
    reset_view()
    apply_rotation()
    moved_camera = true
  end
  if moved_camera then
    apply_camera()
  end
end

local function read_grid(held, pressed)
  local skin = ui.skin
  local direction = nil
  if moved(held, MOVE_LEFT) then
    direction = "left"
  elseif moved(held, MOVE_RIGHT) then
    direction = "right"
  elseif moved(held, MOVE_UP) then
    direction = "up"
  elseif moved(held, MOVE_DOWN) then
    direction = "down"
  end
  if direction == nil then
    view.held_for = 0
    view.held_dir = nil
  else
    local fire = false
    if direction ~= view.held_dir then
      view.held_dir = direction
      view.held_for = 0
      fire = true
    else
      view.held_for = view.held_for + 1
      fire = view.held_for >= REPEAT_DELAY and (view.held_for - REPEAT_DELAY) % REPEAT_EVERY == 0
    end
    if fire then
      if direction == "left" then
        select(view.selected - 1)
      elseif direction == "right" then
        select(view.selected + 1)
      elseif direction == "up" then
        select(math.max(1, view.selected - skin.columns))
      else
        select(math.min(item_count, view.selected + skin.columns))
      end
    end
  end
  local page_number = skin.page_of[view.selected]
  if pressed & BUTTON_L ~= 0 then
    select(skin.pages[math.max(1, page_number - 1)].first, "page")
  elseif pressed & BUTTON_R ~= 0 then
    select(skin.pages[math.min(#skin.pages, page_number + 1)].first, "page")
  end
  if pressed & BUTTON_A ~= 0 then
    sound("decide")
    set_mode("view")
  end
end

local function turn_page(by)
  local page = view.page + by
  if page < 1 then
    page = view.pages
  elseif page > view.pages then
    page = 1
  end
  if page ~= view.page then
    view.page = page
    sound("page")
    draw_description()
  end
end

local function toggle_angle()
  sound("decide")
  view.angle = not view.angle
  reset_view()
  show_parts()
  if view.shown then
    apply_camera()
    apply_rotation()
  end
end

local function read_view_buttons(held, pressed)
  if pressed & (BUTTON_L | DPAD_LEFT) ~= 0 then
    select(view.selected - 1, "page")
  elseif pressed & (BUTTON_R | DPAD_RIGHT) ~= 0 then
    select(view.selected + 1, "page")
  end
  if pressed & BUTTON_A ~= 0 then
    toggle_angle()
  elseif pressed & DPAD_DOWN ~= 0 then
    turn_page(1)
  elseif pressed & DPAD_UP ~= 0 then
    turn_page(-1)
  end
  if pressed & BUTTON_ZR ~= 0 then
    sound("cursor")
    view.help = not view.help
    show_parts()
  end
end

local function load_wanted()
  UiFighterManager.reload_fighter(FIGHTER_ENTRY_ID_SELF, item_text("engine.gallery_fighter", view.wanted), call("engine.gallery_costume", view.wanted - 1))
  view.loaded = view.wanted
  view.sent = false
  view.shown = false
  log("loading", view.wanted, view.frames)
end

local function step()
  view.frames = view.frames + 1
  local held = call("engine.buttons")
  if type(held) ~= "number" then
    return
  end
  if view.frames == 1 then
    view.previous = held
    UiFighterManager.reload_fighter(FIGHTER_ENTRY_ID_SELF, PLACEHOLDER, 0)
  end
  local pressed = held & (~view.previous)
  view.previous = held
  if pressed & BUTTON_B ~= 0 then
    sound("cancel")
    if ui.v2 and view.mode == "view" then
      set_mode("grid")
    else
      view.done = true
    end
    return
  end
  if ui.v2 then
    if view.mode == "grid" then
      read_grid(held, pressed)
    else
      read_view_buttons(held, pressed)
    end
  else
    read_movement(held, pressed)
    read_text(held, pressed)
  end
  local state = call("engine.viewer_state", SLOT)
  if view.placeholder ~= true then
    if state == STATE_READY then
      view.placeholder = true
      load_wanted()
    end
    return
  end
  if view.loaded ~= view.wanted and (state == STATE_READY or state == STATE_SHOWN) then
    load_wanted()
    return
  end
  if state == STATE_READY then
    if view.sent ~= true then
      view.sent = true
      UiFighterManager.activate(FIGHTER_ENTRY_ID_SELF)
    end
  elseif state == STATE_SHOWN then
    if view.shown ~= true then
      view.shown = true
      apply_camera()
      apply_rotation()
    end
    if ui.v2 then
      if view.mode == "view" then
        read_inspect(held, pressed)
      end
    else
      read_view(held, pressed)
    end
  else
    view.sent = false
  end
end

is_finished_in = function()
  return 1
end

get_exit_value = function()
  return -1
end

main = function()
  if item_count == 0 then
    log("no items")
    return
  end
  local ok = pcall(setup)
  if ok ~= true then
    log("this screen has no gallery layout")
    return
  end
  pcall(ComFrameActor.hide_frame, true)
  call("engine.viewer_begin", SLOT)
  local song = text("engine.gallery_music")
  if type(song) == "string" and song ~= "" then
    view.song = pcall(function()
      UiSoundManager:play_bgm(song)
    end)
    log("music", song, view.song)
  end
  if ui.v2 then
    set_mode("grid")
  else
    draw()
  end
  repeat
    if pcall(step) ~= true then
      log("step failed", view.frames)
      view.done = true
    end
    if view.done ~= true then
      coroutine.yield()
    end
  until view.done == true
  call("engine.viewer_end")
  if view.song then
    pcall(function()
      UiSoundManager:play_menu_bgm()
    end)
  end
  pcall(UiFighterManager.deactivate, FIGHTER_ENTRY_ID_SELF)
  ui.page:set_visible(false)
  if ui.skin ~= nil then
    pcall(function()
      ui.skin.page:set_visible(false)
      ui.skin.root:deactivate_layout()
    end)
  end
  pcall(function() ui.root:deactivate_layout() end)
  pcall(ComFrameActor.hide_frame, false)
end
