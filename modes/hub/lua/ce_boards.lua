-- Spirit boards from spirit.toml packs on the DLC Spirits popup. The game's
-- eleven boards stay on the first page; L and R turn to pages of the packs'
-- boards, eleven a page, on the same buttons: their banners give way to the
-- board's name, the art to a card with it, and choosing one opens the board
-- the way the game opens its own (set_selected_dlc_fighter with the board's
-- id, then the special board sequence). A board with art (spirit.toml art, a
-- layout with a banner and a card pane) shows them in place of its name.

local dlc = UiScriptPlayer.require("menu/main/main_menu_layer_dlc")
local game_initialize = dlc.initialize
local game_setup_selector = dlc.setup_selector
local game_update_state_main = dlc.update_state_main
local game_update_state_out = dlc.update_state_out

local PER_PAGE = 11
local SLOTS = {{-123, 263}, {-123, 124}, {-123, -15}, {-123, -154}, {-123, -293},
               {383, 402}, {383, 263}, {383, 124}, {383, -15}, {383, -154}, {383, -293}}
local CARD_X = -610
local CARD_Y = 7
local KEY_PREV = 7
local KEY_NEXT = 8
local PARTS = {}
local IDS = {}

local function call(name, ...)
  local value = UiScriptPlayer.engine_call(name, ...)
  if type(value) ~= "number" then
    return 0
  end
  return value
end

local function text(name, index)
  local read = UiScriptPlayer.engine_text or prg_func.engine_text
  if read == nil then
    return nil
  end
  return read(name, index)
end

local function log(...)
  UiScriptPlayer.engine_call("engine.log", ...)
end

local function held(value)
  return value == true or value == 1
end

local function slot_ids()
  if #IDS > 0 then
    return true
  end
  for slot = 1, PER_PAGE do
    local id = button[string.format("SPIRITS_DLC_FIGHTER%02d", slot - 1)]
    if id == nil then
      return false
    end
    IDS[slot] = id
    PARTS[slot] = string.format("set_parts_btn_%02d", slot - 1)
  end
  return true
end

local function board_of(self, slot)
  if self.ce_page == nil or self.ce_page <= 1 then
    return nil
  end
  return self.ce_boards[(self.ce_page - 2) * PER_PAGE + slot]
end

local function overlay(self)
  if self.ce_view ~= nil then
    return self.ce_view
  end
  local root = LayoutRootList.ce_boards
  if root == nil then
    return nil
  end
  local view = root:get_root_view()
  self.ce_view = view
  return view
end

local function art_of(board)
  if board.art == nil or LayoutRootList[board.art] == nil then
    return
  end
  local view = LayoutRootList[board.art]:get_root_view()
  board.banner = view:get_pane("banner")
  board.card = view:get_pane("card")
  if board.banner ~= nil then
    board.banner:set_visible(false)
  end
  if board.card ~= nil then
    board.card:set_visible(false)
  end
end

local function hide_art(self)
  for _, board in ipairs(self.ce_boards or {}) do
    if board.banner ~= nil then
      board.banner:set_visible(false)
    end
    if board.card ~= nil then
      board.card:set_visible(false)
    end
  end
end

local function caption(self)
  local view = overlay(self)
  if view == nil then
    return
  end
  local line = view:get_pane("page")
  if self.ce_page == 1 then
    line:set_text_string(string.format("R: more boards  1 / %d", self.ce_pages))
  else
    line:set_text_string(string.format("L / R: boards  %d / %d", self.ce_page, self.ce_pages))
  end
  line:set_visible(true)
end

local function show_card(self)
  local view = overlay(self)
  if view == nil then
    return
  end
  local board = nil
  if self.ce_page > 1 and self.selector ~= nil then
    local selected = self.selector:get_selected_button_id()
    for slot = 1, PER_PAGE do
      if IDS[slot] == selected then
        board = board_of(self, slot)
      end
    end
  end
  for _, other in ipairs(self.ce_boards) do
    if other.card ~= nil and other ~= board then
      other.card:set_visible(false)
    end
  end
  if board ~= nil and board.card ~= nil then
    board.card:set_position(CARD_X, CARD_Y)
    board.card:set_visible(true)
    view:get_pane("card"):set_visible(false)
    return
  end
  view:get_pane("card"):set_visible(board ~= nil)
  if board ~= nil then
    view:get_pane("card_name"):set_text_string((string.gsub(board.name, " ", "\n")))
  end
end

local function dress(self)
  local custom = self.ce_page > 1
  self.layout_view:get_pane("preview"):set_visible(not custom)
  local view = overlay(self)
  hide_art(self)
  for slot = 1, PER_PAGE do
    local board = board_of(self, slot)
    local parts = self.layout_view:get_parts(PARTS[slot])
    if parts ~= nil then
      parts:get_pane("image"):set_visible(not custom)
    end
    local banner = board ~= nil and board.banner or nil
    if banner ~= nil then
      banner:set_position(SLOTS[slot][1], SLOTS[slot][2])
      banner:set_visible(true)
    end
    if view ~= nil then
      local name = string.format("board_%02d", slot - 1)
      view:get_pane(name):set_visible(board ~= nil and banner == nil)
      if board ~= nil then
        view:get_pane(name .. "_label"):set_text_string(board.name)
      end
    end
  end
  if view ~= nil then
    view:get_pane("boards"):set_visible(true)
  end
  caption(self)
  show_card(self)
end

local function apply_names(self)
  for slot = 1, PER_PAGE do
    local entry = self.dlc_table[IDS[slot]]
    if entry ~= nil then
      if self.ce_page == 1 then
        entry.name = self.ce_vanilla[slot]
      else
        local board = board_of(self, slot)
        entry.name = board and board.id or nil
      end
    end
  end
end

function dlc:initialize()
  game_initialize(self)
  local ok, failure = pcall(function()
    self.ce_boards = {}
    if not slot_ids() then
      return
    end
    local count = call("engine.spirit_board_count")
    for index = 0, count - 1 do
      local id = text("engine.spirit_board_id", index)
      if id ~= nil then
        local board = {id = id, name = text("engine.spirit_board_name", index) or id,
                       art = text("engine.spirit_board_art", index)}
        art_of(board)
        self.ce_boards[#self.ce_boards + 1] = board
      end
    end
    if #self.ce_boards == 0 then
      return
    end
    self.ce_vanilla = {}
    for slot = 1, PER_PAGE do
      local entry = self.dlc_table[IDS[slot]]
      self.ce_vanilla[slot] = entry and entry.name or nil
    end
    self.ce_pages = 1 + math.ceil(#self.ce_boards / PER_PAGE)
    self.ce_page = math.min(clone_engine_board_page or 1, self.ce_pages)
    self.ce_input = self.layout_root:get_virtual_input()
    apply_names(self)
    log("boards: " .. tostring(#self.ce_boards) .. " board(s) from packs on the DLC Spirits popup")
  end)
  if not ok then
    log("boards: setup failed: " .. tostring(failure))
    self.ce_boards = {}
  end
end

function dlc:setup_selector()
  game_setup_selector(self)
  if self.ce_boards == nil or #self.ce_boards == 0 then
    return
  end
  local ok, failure = pcall(function()
    if self.ce_page > 1 then
      for slot = 1, PER_PAGE do
        local board = board_of(self, slot)
        local parts = self.layout_view:get_parts(PARTS[slot])
        if board ~= nil then
          self.selector:get_button(IDS[slot]):set_disabled(false)
          self.button_table[IDS[slot]].func_arg = {name = board.id, unlock = true}
          self.button_table[IDS[slot]].sequence = SEQUENCE_SPIRITS_CHALLENGE_SPECIAL
        end
        if parts ~= nil then
          common.play_animation(parts, "hide_badge")
        end
      end
    end
    dress(self)
  end)
  if not ok then
    log("boards: page failed: " .. tostring(failure))
  end
end

local function turn(self, step)
  local page = self.ce_page + step
  if page < 1 or page > self.ce_pages then
    return
  end
  self.ce_page = page
  clone_engine_board_page = page
  apply_names(self)
  self:setup_selector()
  for slot = 1, PER_PAGE do
    if self.dlc_table[IDS[slot]] ~= nil and self.dlc_table[IDS[slot]].name ~= nil then
      self.selector:select_button(IDS[slot], true)
      break
    end
  end
  UiSoundManager:play_se_label("se_system_cursor_l")
  show_card(self)
end

function dlc:update_state_main()
  if self.ce_boards ~= nil and #self.ce_boards > 0 and self.ce_input ~= nil then
    local ok, failure = pcall(function()
      if held(self.ce_input:is_pressed(KEY_NEXT)) then
        turn(self, 1)
      elseif held(self.ce_input:is_pressed(KEY_PREV)) then
        turn(self, -1)
      end
      if self.ce_page > 1 then
        show_card(self)
      end
    end)
    if not ok then
      log("boards: input failed: " .. tostring(failure))
      self.ce_boards = {}
    end
  end
  game_update_state_main(self)
end

function dlc:update_state_out()
  pcall(hide_art, self)
  if self.ce_view ~= nil then
    pcall(function()
      self.ce_view:get_pane("boards"):set_visible(false)
      self.ce_view:get_pane("card"):set_visible(false)
    end)
  end
  game_update_state_out(self)
end

return dlc
