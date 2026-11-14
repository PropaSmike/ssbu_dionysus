-- The modes page: one row per [[mode]] the packs' menu.toml declare, opened
-- from any main menu page by the modes button (modes_button in
-- sd:/ultimate/dionysus/config.toml). It is drawn on the game's own Mob
-- Smash popup: three rows that scroll through every installed mode, the
-- selected mode's art where the popup shows its picture, and its description
-- in the footer. Choosing a row starts the mode's sequence, or runs the mode
-- as a screen inside this page (its script's enter / update / leave), so
-- modes never share menu states, codes or buttons. A mode that starts a
-- battle comes back here after it.

-- require runs this file again for every script that requires it: the
-- first copy waits in the menu's globals and the others hand it out.
if clone_engine_modes ~= nil then
  return clone_engine_modes
end

local modes = {}
clone_engine_modes = modes

local STATE_IN = 94
local STATE_MAIN = 95
local STATE_OUT = 96
local STATE_HOST = 97
local ROWS = 3
local PARTS = {"set_parts_btn_00", "set_parts_btn_01", "set_parts_btn_02"}
local ROW_IDS = {0x7fce01, 0x7fce02, 0x7fce03}
local ROW_TEXTS = {"set_txt", "set_txt_disable"}
local ROW_ICONS = {"set_icon_off", "set_icon_on", "set_icon_off_disable", "set_icon_on_disable"}
-- What Mob Smash shows, put back when the page closes.
local VANILLA_ROWS = {"mnu_oth_pop_spar_hundred", "mnu_oth_pop_spar_allstar", "mnu_oth_pop_spar_ruthless"}
local VANILLA_TITLE = "mnu_oth_pop_spar_title"
local TITLE = "Modes"
local ART_X = -584
local ART_Y = 7
local KEY_UP = 0
local KEY_DOWN = 1
local KEY_LEFT = 2
local KEY_RIGHT = 3
local KEY_OK = 4
local KEY_CANCEL = 5
local KEY_HELP = 6
local KEY_PREV = 7
local KEY_NEXT = 8

local function call(name, ...)
  local value = prg_func.engine_call(name, ...)
  if type(value) ~= "number" then
    return -1
  end
  return value
end

local function text(name, index)
  if prg_func.engine_text == nil then
    return nil
  end
  return prg_func.engine_text(name, index)
end

local function log(...)
  prg_func.engine_call("engine.log", ...)
end

local function decide_sound()
  local ok, id = pcall(function()
    return UI_SE_ID_DECIDE
  end)
  if ok and type(id) == "number" and pcall(UiSoundManager.play_se, id) then
    return
  end
  pcall(function()
    UiSoundManager:play_se_label("se_system_fixed_s")
  end)
end

local function held(value)
  return value == true or value == 1
end

local function read_keys(input)
  local keys = {
    up = held(input:is_pressed(KEY_UP)),
    down = held(input:is_pressed(KEY_DOWN)),
    left = held(input:is_pressed(KEY_LEFT)),
    right = held(input:is_pressed(KEY_RIGHT)),
    ok = held(input:is_pressed(KEY_OK)),
    cancel = held(input:is_pressed(KEY_CANCEL)),
    help = held(input:is_pressed(KEY_HELP)),
    prev = held(input:is_pressed(KEY_PREV)),
    next = held(input:is_pressed(KEY_NEXT)),
  }
  if not keys.up and not keys.down then
    keys.up = held(input:is_repeated(KEY_UP))
    keys.down = not keys.up and held(input:is_repeated(KEY_DOWN))
  end
  if not keys.left and not keys.right then
    keys.left = held(input:is_repeated(KEY_LEFT))
    keys.right = not keys.left and held(input:is_repeated(KEY_RIGHT))
  end
  return keys
end

-- A Lua error inside a main menu state stops the whole menu: a page state
-- that fails is logged and the page closes instead.
local function guarded(step, fallback)
  return function(self)
    local ok, failure = pcall(step, self)
    if ok then
      return
    end
    log("modes: page failed: " .. tostring(failure))
    if fallback ~= nil then
      state:set(fallback)
    elseif not state:pop() then
      root_type = SEQUENCE_TITLE
      state:set(STATE_TERM)
    end
  end
end

local function row_of(id)
  for row, row_id in ipairs(ROW_IDS) do
    if row_id == id then
      return row
    end
  end
  return nil
end

-- What a mode's script gets in decide, enter, update and leave.
local Host = {}

function Host:keys()
  return self.current_keys or {}
end

function Host:pad()
  return modes.input:get_last_input_device_id()
end

-- Back to the modes page (leave runs first).
function Host:close()
  self.closing = true
end

-- Ends the main menu and starts `code` (the mode's own sequence, or any
-- [[sequence]] of its pack, by id or code); the menu comes back to this mode
-- afterwards.
function Host:leave_for(code)
  if type(code) == "string" then
    code = call("engine.sequence", code)
  end
  if type(code) ~= "number" or code <= 0 then
    return false
  end
  call("engine.mode_return_to", self.index)
  modes:release_selector()
  modes.root:set_enable_input(false)
  root_type = code
  state:set(STATE_TERM)
  self.left = true
  return true
end

-- The code the engine gave the mode's runs_as, 0 without one.
function Host:sequence()
  return call("engine.mode_sequence", self.index)
end

function Host:start()
  local code = self:sequence()
  if code > 0 then
    self:leave_for(code)
    return true
  end
  return false
end

-- A number the engine keeps for the mode while the game runs (a cursor to
-- come back to after a battle).
function Host:value()
  return call("engine.mode_value", self.index)
end

function Host:set_value(value)
  call("engine.mode_set_value", self.index, value)
end

local function new_host(entry)
  local host = {index = entry.index, id = entry.id, name = entry.name, resumed = false}
  for name, method in pairs(Host) do
    host[name] = method
  end
  return host
end

function modes:setup()
  if self.ready ~= nil then
    return self.ready
  end
  self.ready = false
  local root = LayoutRootList.other_popup
  if root == nil then
    return false
  end
  self.root = root
  self.view = root:get_root_view()
  self.input = root:get_virtual_input()
  if LayoutRootList.ce_modes ~= nil then
    local card_view = LayoutRootList.ce_modes:get_root_view()
    self.card = card_view:get_pane("card")
    self.card_name = card_view:get_pane("card_name")
    self.card:set_visible(false)
  end
  self.entries = {}
  local count = call("engine.mode_count")
  for index = 0, count - 1 do
    local entry = {
      index = index,
      id = text("engine.mode_id", index) or "",
      name = text("engine.mode_name", index) or "",
      description = text("engine.mode_description", index) or "",
      script = text("engine.mode_script", index),
      button = text("engine.mode_button", index),
      sequence = call("engine.mode_sequence", index),
    }
    if entry.button ~= nil and LayoutRootList[entry.button] ~= nil then
      local art = LayoutRootList[entry.button]:get_root_view():get_pane("RootPane")
      if art ~= nil then
        art:set_visible(false)
        entry.art = art
      end
    end
    self.entries[#self.entries + 1] = entry
  end
  state:setup(STATE_IN, self, guarded(self.update_in, STATE_OUT), "STATE_CE_MODES_IN")
  state:setup(STATE_MAIN, self, guarded(self.update_main, STATE_OUT), "STATE_CE_MODES")
  state:setup(STATE_OUT, self, guarded(self.update_out), "STATE_CE_MODES_OUT")
  state:setup(STATE_HOST, self, guarded(self.update_host, STATE_IN), "STATE_CE_MODES_HOST")
  for _, entry in ipairs(self.entries) do
    self:load(entry)
  end
  self.cursor = 1
  self.top = 1
  self.blurred = false
  self.ready = true
  log("modes: " .. tostring(#self.entries) .. " mode(s) set up")
  return true
end

-- Runs one of a mode's functions; a mode that raises an error is logged and
-- sent back to the rows instead of stopping the main menu.
function modes:run(entry, name, ...)
  local fn = entry.module ~= nil and entry.module[name] or nil
  if fn == nil then
    return true, nil
  end
  if pcall == nil then
    return true, fn(...)
  end
  local ok, result = pcall(fn, ...)
  if not ok then
    log("mode " .. entry.id .. " " .. name .. " failed: " .. tostring(result))
  end
  return ok, result
end

function modes:load(entry)
  if entry.loaded then
    return entry.module
  end
  entry.loaded = true
  if entry.script == nil then
    return nil
  end
  -- require gives the script the environment of the Lua function calling
  -- it: called straight from pcall (a C function), the script's globals
  -- were nil, so it is called from a closure here.
  local function load_script()
    return UiScriptPlayer.require(entry.script)
  end
  local ok, module = true, nil
  if pcall ~= nil then
    ok, module = pcall(load_script)
  else
    module = load_script()
  end
  if not ok or type(module) ~= "table" then
    log("mode " .. entry.id .. ": " .. entry.script .. " did not load: " .. tostring(module))
    return nil
  end
  entry.module = module
  log("mode " .. entry.id .. ": " .. entry.script .. " loaded")
  return module
end

function modes:hide_art()
  for _, entry in ipairs(self.entries) do
    if entry.art ~= nil then
      entry.art:set_visible(false)
    end
  end
  if self.card ~= nil then
    self.card:set_visible(false)
  end
end

-- The selected mode's art where the popup shows its picture; a mode without
-- art gets a card with its name, one word a line.
function modes:show_art()
  self:hide_art()
  local entry = self.entries[self.cursor]
  if entry == nil then
    return
  end
  if entry.art ~= nil then
    entry.art:set_position(ART_X, ART_Y)
    entry.art:set_visible(true)
  elseif self.card ~= nil then
    self.card_name:set_text_string((string.gsub(entry.name, " ", "\n")))
    self.card:set_visible(true)
  end
end

function modes:caption()
  local count = #self.entries
  local title = TITLE
  if count > ROWS then
    title = string.format("%s   %d / %d", TITLE, self.cursor, count)
  end
  self.view:get_pane("txt_replay"):set_text_string(title)
end

-- Puts the modes on the popup's rows, from the row `self.top` on.
function modes:fill()
  for row = 1, ROWS do
    local entry = self.entries[self.top + row - 1]
    self.view:get_pane(PARTS[row]):set_visible(entry ~= nil)
    if entry ~= nil then
      local part = self.view:get_parts(PARTS[row])
      for _, name in ipairs(ROW_TEXTS) do
        part:get_pane(name):set_text_string(entry.name)
      end
      for _, name in ipairs(ROW_ICONS) do
        part:get_pane(name):set_visible(false)
      end
    end
  end
end

function modes:dress()
  self.view:get_pane("pic_image"):set_visible(false)
  self:fill()
  self:caption()
end

-- Mob Smash opens this same popup: its texts, icons and picture go back.
function modes:undress()
  self.view:get_pane("txt_replay"):set_text_message(VANILLA_TITLE)
  self.view:get_pane("pic_image"):set_visible(true)
  for row = 1, ROWS do
    self.view:get_pane(PARTS[row]):set_visible(true)
    local part = self.view:get_parts(PARTS[row])
    for _, name in ipairs(ROW_TEXTS) do
      part:get_pane(name):set_text_message(VANILLA_ROWS[row])
    end
    for _, name in ipairs(ROW_ICONS) do
      part:get_pane(name):set_visible(true)
    end
  end
end

function modes:open_selector()
  self:release_selector()
  local config = LayoutButtonSelectorConfig.new()
  config.directional_degree = 60
  config.use_cursor_looping = false
  config.is_unique_se = true
  config.decide_se_label_code = "se_system_fixed_s"
  config.cancel_se_label_code = "se_system_cancel"
  config.cursor_se_label_code = "se_system_cursor_l"
  local selector = LayoutButtonSelector.new()
  selector:setup(self.view, "selector_0", config)
  for row = 1, ROWS do
    if self.entries[self.top + row - 1] ~= nil then
      selector:setup_button(ROW_IDS[row], PARTS[row])
    end
  end
  selector:set_enable(true)
  selector:set_focus(true)
  self.row = self.cursor - self.top + 1
  selector:select_button(ROW_IDS[self.row], true)
  self.selector = selector
end

function modes:release_selector()
  if self.selector ~= nil and self.selector:is_valid() then
    self.selector:release()
  end
  self.selector = nil
end

function modes:blur(on)
  if on == self.blurred then
    return
  end
  self.blurred = on
  if on then
    UiGraphicsManager.begin_screen_blur()
  else
    UiGraphicsManager.end_screen_blur()
  end
end

-- The footer shows the selected mode's description, and the page's own
-- help again once the modes page closes.
function modes:describe(entry)
  if footer == nil or footer.in_pane == nil then
    return
  end
  local description = entry ~= nil and entry.description or ""
  if self.described == nil then
    footer:set_label(footer.out_pane, footer.prev_label)
  else
    footer.out_pane:set_text_string(self.described)
  end
  footer.in_pane:set_text_string(description)
  footer:change()
  self.described = description
end

function modes:undescribe()
  if footer == nil or footer.in_pane == nil or self.described == nil then
    return
  end
  footer.out_pane:set_text_string(self.described)
  footer:set_label(footer.in_pane, footer.prev_label)
  footer:change()
  self.described = nil
end

-- From common.update_state_main: opens the page on the modes button, or
-- right away when the menu comes back from a mode's sequence.
function modes:take_over(layer)
  if not self:setup() or #self.entries == 0 then
    return false
  end
  -- Mob Smash itself is open on the popup: the page waits until it closes.
  if layer ~= nil and layer.layout_root == self.root then
    return false
  end
  if not self.primed then
    self.primed = true
    call("engine.mode_pressed")
    local back = call("engine.mode_take_resume")
    if back >= 0 and back < #self.entries then
      -- main_menu.initialize updates the menu in a loop until exec_resume
      -- clears resume; the page takes the first frame instead, so it ends
      -- that loop itself and shows the footer the frame would have shown.
      resume = nil
      if footer ~= nil then
        footer:appear()
      end
      self:aim(back + 1)
      self.resume_entry = self.entries[back + 1]
      self:open(layer)
      return true
    end
    return false
  end
  if call("engine.mode_pressed") ~= 1 then
    return false
  end
  UiSoundManager:play_se_label("se_system_fixed_s")
  self:open(layer)
  return true
end

function modes:open(layer)
  if layer ~= nil and layer.layout_root ~= nil then
    layer.layout_root:set_enable_input(false)
  end
  state:push(STATE_IN)
end

-- Selects mode `index`, scrolling the rows so it shows.
function modes:aim(index)
  local count = #self.entries
  index = math.max(1, math.min(index, count))
  self.cursor = index
  if index < self.top then
    self.top = index
  elseif index > self.top + ROWS - 1 then
    self.top = index - ROWS + 1
  end
  self.top = math.max(1, math.min(self.top, math.max(count - ROWS + 1, 1)))
end

function modes:enter(entry, resumed)
  local host = new_host(entry)
  host.resumed = resumed
  self.active = entry
  self.host = host
  state:set(STATE_HOST)
end

function modes:choose(entry)
  log("modes: choose " .. entry.id)
  local module = self:load(entry)
  if module == nil then
    decide_sound()
  end
  if module ~= nil and module.enter ~= nil then
    self:enter(entry, false)
    return
  end
  if entry.sequence > 0 then
    local host = new_host(entry)
    local ok, result = self:run(entry, "decide", host)
    if not ok or result == false then
      self:open_selector()
      return
    end
    host:leave_for(entry.sequence)
    return
  end
  log("mode " .. entry.id .. " has nothing to run")
  self:open_selector()
end

function modes:update_in()
  local resume_entry = self.resume_entry
  self.resume_entry = nil
  if resume_entry ~= nil then
    local module = self:load(resume_entry)
    if module ~= nil and module.enter ~= nil then
      self:enter(resume_entry, true)
      return
    end
  end
  if state:is_first() then
    self.root:set_enable_input(true)
    self:dress()
    self.view:play_animation("in", 1)
    self:open_selector()
    self:blur(true)
    if footer ~= nil then
      footer:disappear()
    end
    self.shown = false
    self.art_shown = false
  end
  state:set(STATE_MAIN)
end

function modes:update_main()
  -- main_menu.init_state turns every layout's input off at each state change
  -- but the input of the vanilla page the state belongs to.
  if state:is_first() then
    self.root:set_enable_input(true)
  end
  if not self.shown then
    self.shown = true
    if footer ~= nil then
      footer:appear()
    end
    self:describe(self.entries[self.cursor])
  end
  -- The art is a layout of its own: it comes once the popup is in.
  if not self.art_shown and self.view:is_animation_finished("in") then
    self.art_shown = true
    self:show_art()
  end
  local selector = self.selector
  if selector == nil then
    return
  end
  local count = #self.entries
  local keys = read_keys(self.input)
  local row = row_of(selector:get_selected_button_id()) or self.row
  if selector:is_cancelled() then
    state:set(STATE_OUT)
    return
  end
  if selector:get_decided_button_id() ~= BUTTON_ID_NONE and row ~= nil then
    self.cursor = self.top + row - 1
    local entry = self.entries[self.cursor]
    if entry ~= nil then
      self:choose(entry)
    end
    return
  end
  -- The selector moves inside the three rows; past the first or last one
  -- the rows scroll instead, and L / R jump three modes.
  local cursor = self.top + row - 1
  if row == self.row then
    if keys.up and row == 1 and self.top > 1 then
      cursor = cursor - 1
    elseif keys.down and row == ROWS and self.top + ROWS <= count then
      cursor = cursor + 1
    end
  end
  if keys.prev then
    cursor = math.max(cursor - ROWS, 1)
  elseif keys.next then
    cursor = math.min(cursor + ROWS, count)
  end
  self.row = row
  if cursor == self.cursor then
    return
  end
  local top = self.top
  self:aim(cursor)
  if self.top ~= top then
    self:fill()
  end
  if self.top ~= top or keys.prev or keys.next then
    UiSoundManager:play_se_label("se_system_cursor_l")
    self.row = self.cursor - self.top + 1
    selector:select_button(ROW_IDS[self.row], true)
  end
  self:caption()
  if self.art_shown then
    self:show_art()
  end
  self:describe(self.entries[self.cursor])
end

function modes:update_host()
  local entry, host = self.active, self.host
  if entry == nil or host == nil then
    state:set(STATE_IN)
    return
  end
  if state:is_first() then
    log("modes: enter " .. entry.id)
    self:release_selector()
    self:hide_art()
    self:undescribe()
    self:blur(false)
    self.view:stop_animation_at_end("out")
    self.root:set_enable_input(true)
    if not self:run(entry, "enter", host, host.resumed) then
      host.closing = true
    end
  end
  if host.left then
    return
  end
  if not host.closing then
    host.current_keys = read_keys(self.input)
    if not self:run(entry, "update", host, host.current_keys) then
      host.closing = true
    end
  end
  if host.left then
    return
  end
  if host.closing then
    log("modes: leave " .. entry.id)
    self:run(entry, "leave", host)
    self.active = nil
    self.host = nil
    state:set(STATE_IN)
  end
end

function modes:update_out()
  if state:is_first() then
    self:release_selector()
    self:hide_art()
    self:undescribe()
    self:blur(false)
    self.root:set_enable_input(false)
    self.view:play_animation("out", 1)
  end
  if not self.view:is_animation_finished("out") then
    return
  end
  self:undress()
  if not state:pop() then
    root_type = SEQUENCE_TITLE
    state:set(STATE_TERM)
  end
end

return modes
