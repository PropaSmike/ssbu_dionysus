use crate::{
    article_index, article_status, arm_kirby_copy_status_family, copy_article_index, elog,
    entry_kind, kind_for_identity, param_int_override_full, param_override_full, resolve,
    shared_hook_checked, ArticleStatusLine, Error, HookFn, ParamOp, SharedHookRegistrationV1,
    RESULT_OK,
};
use std::ffi::{c_void, CString};
use std::sync::atomic::{AtomicI32, AtomicU64, AtomicUsize, Ordering};

pub use dionysus_macros::hook;
pub use crate::manifest::{
    register_fighter_manifest, register_item_manifest, ArticleLvd, Feature, Grab, Hurtbox, ItemManifest,
    LvdVertex, Manifest, Motion,
};
pub use crate::slot;
pub use crate::{HudAction as Hud, MenuCommand};
pub use crate::ArticleStatusLine as Line;
pub use crate::{fighter, install_hooks, item};

const UNRESOLVED: i32 = i32::MIN;

#[derive(Clone, Copy, Debug)]
pub struct Kind(Option<i32>);

impl Kind {
    pub fn get(self) -> Option<i32> {
        self.0
    }

    pub fn is_some(self) -> bool {
        self.0.is_some()
    }
}

impl PartialEq<i32> for Kind {
    fn eq(&self, other: &i32) -> bool {
        self.0 == Some(*other)
    }
}

impl PartialEq<Kind> for i32 {
    fn eq(&self, other: &Kind) -> bool {
        other.0 == Some(*self)
    }
}

impl core::fmt::Display for Kind {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.0 {
            Some(kind) => write!(formatter, "{kind}"),
            None => formatter.write_str("unallocated"),
        }
    }
}

pub trait AsBoma {
    fn as_boma(&self) -> *mut c_void;
}

impl AsBoma for u64 {
    fn as_boma(&self) -> *mut c_void {
        *self as usize as *mut c_void
    }
}

impl AsBoma for usize {
    fn as_boma(&self) -> *mut c_void {
        *self as *mut c_void
    }
}

impl AsBoma for *mut c_void {
    fn as_boma(&self) -> *mut c_void {
        *self
    }
}

impl AsBoma for *const c_void {
    fn as_boma(&self) -> *mut c_void {
        *self as *mut c_void
    }
}

#[cfg(feature = "smash")]
mod smash_boma {
    use super::AsBoma;
    use std::ffi::c_void;

    impl AsBoma for *mut smash::app::BattleObjectModuleAccessor {
        fn as_boma(&self) -> *mut c_void {
            self.cast()
        }
    }

    impl AsBoma for *const smash::app::BattleObjectModuleAccessor {
        fn as_boma(&self) -> *mut c_void {
            *self as *mut c_void
        }
    }

    impl AsBoma for *mut smash::app::BattleObject {
        fn as_boma(&self) -> *mut c_void {
            if self.is_null() {
                return core::ptr::null_mut();
            }
            unsafe { (**self).module_accessor.cast() }
        }
    }

    impl AsBoma for *mut smash::app::Fighter {
        fn as_boma(&self) -> *mut c_void {
            if self.is_null() {
                return core::ptr::null_mut();
            }
            unsafe { (**self).battle_object.module_accessor.cast() }
        }
    }

    impl AsBoma for *mut smash::app::Weapon {
        fn as_boma(&self) -> *mut c_void {
            if self.is_null() {
                return core::ptr::null_mut();
            }
            unsafe { (**self).battle_object.module_accessor.cast() }
        }
    }

    impl AsBoma for &smash::lua2cpp::L2CAgentBase {
        fn as_boma(&self) -> *mut c_void {
            self.module_accessor.cast()
        }
    }

    impl AsBoma for &mut smash::lua2cpp::L2CAgentBase {
        fn as_boma(&self) -> *mut c_void {
            self.module_accessor.cast()
        }
    }

    impl AsBoma for &smash::lua2cpp::L2CFighterCommon {
        fn as_boma(&self) -> *mut c_void {
            self.module_accessor.cast()
        }
    }

    impl AsBoma for &mut smash::lua2cpp::L2CFighterCommon {
        fn as_boma(&self) -> *mut c_void {
            self.module_accessor.cast()
        }
    }

    impl AsBoma for &smash::lua2cpp::L2CWeaponCommon {
        fn as_boma(&self) -> *mut c_void {
            self.module_accessor.cast()
        }
    }

    impl AsBoma for &mut smash::lua2cpp::L2CWeaponCommon {
        fn as_boma(&self) -> *mut c_void {
            self.module_accessor.cast()
        }
    }
}

pub struct Fighter {
    identity: &'static str,
    kind: AtomicI32,
}

impl Fighter {
    pub const fn new(identity: &'static str) -> Self {
        Self {
            identity,
            kind: AtomicI32::new(UNRESOLVED),
        }
    }

    pub fn identity(&self) -> &'static str {
        self.identity
    }

    pub fn kind(&self) -> Kind {
        let cached = self.kind.load(Ordering::Acquire);
        if cached != UNRESOLVED {
            return Kind(Some(cached));
        }
        let found = kind_for_identity(self.identity)
            .or_else(|| kind_for_identity(&format!("fighter_kind_{}", self.identity)));
        if let Some(kind) = found {
            self.kind.store(kind, Ordering::Release);
        }
        Kind(found)
    }

    pub fn register(&self, manifest: Manifest) -> Result<i32, Error> {
        if manifest.name() != self.identity {
            elog!(
                "[clone_engine] {}: the manifest is for {:?}; the identity and the manifest name must agree",
                self.identity,
                manifest.name()
            );
            return Err(Error::InvalidName);
        }
        if let Some(kind) = self.kind().get() {
            elog!(
                "[clone_engine] {}: already registered as kind {kind} (a fighter.toml in the pack?); the manifest is ignored",
                self.identity
            );
            return Ok(kind);
        }
        let built = crate::compiled_capabilities();
        let supported = |feature: Feature| built & crate::feature_capability(feature) != 0;
        for feature in manifest.features().into_iter().filter(|feature| !supported(*feature)) {
            elog!(
                "[clone_engine] {}: this Dionysus has no {}, so it is left out",
                self.identity,
                feature.describe()
            );
        }
        let kind = register_fighter_manifest(self.identity, &manifest.to_toml_with(supported))?;
        self.kind.store(kind, Ordering::Release);
        Ok(kind)
    }

    pub fn ready(&self) -> bool {
        self.kind().is_some()
    }

    pub fn is<T: AsBoma>(&self, object: T) -> bool {
        let Some(kind) = self.kind().get() else {
            return false;
        };
        let boma = object.as_boma();
        if boma.is_null() {
            return false;
        }
        #[cfg(feature = "smash")]
        {
            unsafe { crate::true_kind(boma) == kind }
        }
        #[cfg(not(feature = "smash"))]
        {
            crate::article_owner_kind(boma as u64) == kind
        }
    }

    pub fn owns<T: AsBoma>(&self, object: T) -> bool {
        let Some(kind) = self.kind().get() else {
            return false;
        };
        let boma = object.as_boma();
        if boma.is_null() {
            return false;
        }
        #[cfg(feature = "smash")]
        {
            unsafe { crate::owner_true_kind(boma) == kind }
        }
        #[cfg(not(feature = "smash"))]
        {
            crate::article_owner_kind(boma as u64) == kind
        }
    }

    pub fn is_or_owns<T: AsBoma>(&self, object: T) -> bool {
        self.owns(object)
    }

    pub fn entries(&self) -> Vec<i32> {
        let Some(kind) = self.kind().get() else {
            return Vec::new();
        };
        (0..8).filter(|entry| entry_kind(*entry) == kind).collect()
    }

    pub fn in_match(&self) -> bool {
        !self.entries().is_empty()
    }

    pub fn hud<T: AsBoma>(&self, object: T, action: Hud) -> bool {
        if self.kind().get().is_none() {
            return false;
        }
        let boma = object.as_boma();
        if boma.is_null() {
            return false;
        }
        match unsafe { crate::hud(boma, &action) } {
            Ok(()) => true,
            Err(error) => {
                if HUD_FAILURES.fetch_add(1, Ordering::Relaxed) == 0 {
                    elog!(
                        "[clone_engine] {}: hud {} failed: {error:?} (later failures are not logged)",
                        self.identity,
                        action.name()
                    );
                }
                false
            }
        }
    }

    pub fn article(&'static self, name: &str) -> Article {
        Article::new(self, name, false)
    }

    pub fn copy_article(&'static self, name: &str) -> Article {
        Article::new(self, name, true)
    }

    pub fn param(&self, name: &str) -> Param<'_> {
        Param {
            fighter: self,
            name: name.to_string(),
            subparam: String::new(),
            slot: crate::ANY_SLOT,
        }
    }

    pub fn kirby_family(&self) -> Option<(i32, i32)> {
        let kind = self.kind().get()?;
        let address = resolve(
            &KIRBY_FAMILY_FN,
            b"clone_engine_kirby_copy_status_family_v1\0",
        )?;
        let function: KirbyFamilyFn = unsafe { core::mem::transmute(address) };
        let mut first = -1;
        let mut count = 0;
        let result = unsafe { function(kind, &mut first, &mut count) };
        (result == RESULT_OK && count > 0).then_some((first, count))
    }

    pub fn own_status(&self, index: i32) -> i32 {
        let start = self
            .kind()
            .get()
            .map(crate::base_kind)
            .and_then(dionysus_core::status_starts::status_start_of_kind);
        match start {
            Some(start) if index >= 0 => start + index,
            _ => {
                elog!(
                    "[clone_engine] {}: own_status({index}) needs a registered fighter on a vanilla base",
                    self.identity
                );
                -1
            }
        }
    }

    pub fn kirby_status(&self, index: i32) -> i32 {
        match self.kirby_family() {
            Some((first, count)) if index >= 0 && index < count => first + index,
            Some((first, count)) => {
                elog!(
                    "[clone_engine] {}: kirby_status({index}) is outside the family {first:#x}+{count}",
                    self.identity
                );
                -1
            }
            None => {
                elog!("[clone_engine] {}: no Kirby copy status family", self.identity);
                -1
            }
        }
    }

    pub fn arm_kirby(&self) -> bool {
        let (Some(kind), Some((first, count))) = (self.kind().get(), self.kirby_family()) else {
            return false;
        };
        arm_kirby_copy_status_family(kind, first, count)
    }
}

type KirbyFamilyFn = unsafe extern "C" fn(i32, *mut i32, *mut i32) -> i32;
static KIRBY_FAMILY_FN: AtomicUsize = AtomicUsize::new(0);
static HUD_FAILURES: AtomicUsize = AtomicUsize::new(0);

type ArticleKindByNameFn = unsafe extern "C" fn(i32, *const std::os::raw::c_char, u32) -> i32;
static ARTICLE_KIND_BY_NAME_FN: AtomicUsize = AtomicUsize::new(0);

type MountListenerFn = unsafe extern "C" fn(usize) -> i32;
static MOUNT_LISTENER_FN: AtomicUsize = AtomicUsize::new(0);

pub fn on_mods_mounted(callback: extern "C" fn()) -> Result<(), Error> {
    let address = resolve(&MOUNT_LISTENER_FN, b"clone_engine_on_mods_mounted_v1\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: MountListenerFn = unsafe { core::mem::transmute(address) };
    let result = unsafe { function(callback as usize) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub struct Article {
    fighter: &'static Fighter,
    name: String,
    kirby: bool,
    lines: Vec<(ArticleStatusLine, i32, *const ())>,
}

impl Article {
    fn new(fighter: &'static Fighter, name: &str, kirby: bool) -> Self {
        Self {
            fighter,
            name: name.to_string(),
            kirby,
            lines: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn weapon_kind(&self) -> Option<i32> {
        let kind = self.fighter.kind().get()?;
        let name = CString::new(self.name.as_str()).ok()?;
        let address = resolve(
            &ARTICLE_KIND_BY_NAME_FN,
            b"clone_engine_article_kind_by_name_v1\0",
        )?;
        let function: ArticleKindByNameFn = unsafe { core::mem::transmute(address) };
        let weapon = unsafe { function(kind, name.as_ptr(), u32::from(self.kirby)) };
        (weapon >= 0).then_some(weapon)
    }

    pub fn index(&self) -> Option<i32> {
        let kind = self.fighter.kind().get()?;
        let weapon = self.weapon_kind()?;
        if self.kirby {
            copy_article_index(kind, weapon)
        } else {
            article_index(kind, weapon)
        }
    }

    pub fn status_raw(mut self, line: ArticleStatusLine, status: i32, function: *const ()) -> Self {
        self.lines.push((line, status, function));
        self
    }

    #[cfg(feature = "smash")]
    pub fn status(
        self,
        line: ArticleStatusLine,
        status: i32,
        function: unsafe extern "C" fn(&mut smash::lua2cpp::L2CWeaponCommon) -> smash::lib::L2CValue,
    ) -> Self {
        self.status_raw(line, status, function as *const ())
    }

    pub fn install(self) -> bool {
        let Some(weapon) = self.weapon_kind() else {
            elog!(
                "[clone_engine] {}: article {:?} is not registered; add it to fighter.toml",
                self.fighter.identity,
                self.name
            );
            return false;
        };
        let mut ok = true;
        for (line, status, function) in &self.lines {
            if let Err(error) = article_status(weapon, *line, *status, *function) {
                elog!(
                    "[clone_engine] {}: article {:?} status {status:#x} {line:?} refused: {error:?}",
                    self.fighter.identity,
                    self.name
                );
                ok = false;
            }
        }
        ok
    }

    #[cfg(feature = "smash")]
    pub unsafe fn spawn<T: AsBoma>(&self, owner: T) -> bool {
        let Some(index) = self.index() else {
            elog!(
                "[clone_engine] {}: article {:?} is not in the active table",
                self.fighter.identity,
                self.name
            );
            return false;
        };
        let boma = owner.as_boma();
        if boma.is_null() {
            return false;
        }
        smash::app::lua_bind::ArticleModule::generate_article(
            boma.cast::<smash::app::BattleObjectModuleAccessor>(),
            index,
            false,
            0,
        );
        true
    }
}

pub struct Param<'a> {
    fighter: &'a Fighter,
    name: String,
    subparam: String,
    slot: i32,
}

impl Param<'_> {
    pub fn sub(mut self, subparam: &str) -> Self {
        self.subparam = subparam.to_string();
        self
    }

    pub fn slot(mut self, slot: i32) -> Self {
        self.slot = slot;
        self
    }

    fn apply(&self, op: ParamOp, value: f64) -> bool {
        let Some(kind) = self.fighter.kind().get() else {
            return false;
        };
        param_override_full(kind, self.slot, &self.name, &self.subparam, op, value)
    }

    pub fn set(self, value: f64) -> bool {
        self.apply(ParamOp::Set, value)
    }

    pub fn mul(self, value: f64) -> bool {
        self.apply(ParamOp::Mul, value)
    }

    pub fn int(self, value: i32) -> bool {
        let Some(kind) = self.fighter.kind().get() else {
            return false;
        };
        param_int_override_full(kind, self.slot, &self.name, &self.subparam, value)
    }
}

pub trait FromRaw {
    fn from_raw(raw: u64) -> Self;
}

pub trait IntoRaw {
    fn into_raw(self) -> u64;
}

macro_rules! raw_integer {
    ($($ty:ty),*) => {
        $(
            impl FromRaw for $ty {
                fn from_raw(raw: u64) -> Self {
                    raw as $ty
                }
            }

            impl IntoRaw for $ty {
                fn into_raw(self) -> u64 {
                    self as u64
                }
            }
        )*
    };
}

raw_integer!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);

impl FromRaw for bool {
    fn from_raw(raw: u64) -> Self {
        raw & 0xff != 0
    }
}

impl IntoRaw for bool {
    fn into_raw(self) -> u64 {
        u64::from(self)
    }
}

impl FromRaw for () {
    fn from_raw(_raw: u64) {}
}

impl IntoRaw for () {
    fn into_raw(self) -> u64 {
        0
    }
}

impl<T> FromRaw for *mut T {
    fn from_raw(raw: u64) -> Self {
        raw as usize as *mut T
    }
}

impl<T> IntoRaw for *mut T {
    fn into_raw(self) -> u64 {
        self as usize as u64
    }
}

impl<T> FromRaw for *const T {
    fn from_raw(raw: u64) -> Self {
        raw as usize as *const T
    }
}

impl<T> IntoRaw for *const T {
    fn into_raw(self) -> u64 {
        self as usize as u64
    }
}

type VtableEntryFn = unsafe extern "C" fn(i32, u32, u32, u32) -> u64;
static VTABLE_ENTRY_FN: AtomicUsize = AtomicUsize::new(0);
type VtableOverrideFn = unsafe extern "C" fn(i32, u32, u32, u64) -> u64;
static VTABLE_OVERRIDE_FN: AtomicUsize = AtomicUsize::new(0);
type VtableOverride2Fn = unsafe extern "C" fn(i32, u32, u32, u64, u64) -> u32;
static VTABLE_OVERRIDE_2_FN: AtomicUsize = AtomicUsize::new(0);

pub fn get_agent_virtual_function(kind: i32, index: usize, is_weapon: bool, get_ptr: bool) -> usize {
    let Some(address) = resolve(&VTABLE_ENTRY_FN, b"clone_engine_vtable_entry_v1\0") else {
        return 0;
    };
    let Ok(index) = u32::try_from(index) else {
        return 0;
    };
    let function: VtableEntryFn = unsafe { core::mem::transmute(address) };
    unsafe { function(kind, index, u32::from(is_weapon), u32::from(get_ptr)) as usize }
}

pub fn set_vtable_entry(kind: i32, index: u32, is_weapon: bool, function: *const ()) -> Option<usize> {
    set_vtable_entry_in(kind, index, u32::from(is_weapon), function)
}

type CommonVtableOverrideFn = unsafe extern "C" fn(i32, u32, u32, usize) -> usize;
static COMMON_VTABLE_OVERRIDE_FN: AtomicUsize = AtomicUsize::new(0);

pub fn set_common_vtable_entry(kind: i32, slot: u32, function: *const ()) -> Option<usize> {
    set_common_vtable_entry_in(kind, slot, SPACE_FIGHTER, function)
}

pub fn set_common_vtable_entry_in(
    kind: i32,
    slot: u32,
    space: u32,
    function: *const (),
) -> Option<usize> {
    let address = resolve(
        &COMMON_VTABLE_OVERRIDE_FN,
        b"clone_engine_common_vtable_override_v1\0",
    )?;
    let set: CommonVtableOverrideFn = unsafe { core::mem::transmute(address) };
    let previous = unsafe { set(kind, slot, space, function as usize) };
    (previous != 0).then_some(previous)
}

pub const SPACE_FIGHTER: u32 = 0;
pub const SPACE_WEAPON: u32 = 1;
pub const SPACE_ITEM: u32 = 2;

pub fn set_vtable_entry_in(kind: i32, index: u32, space: u32, function: *const ()) -> Option<usize> {
    let address = resolve(&VTABLE_OVERRIDE_FN, b"clone_engine_vtable_override_v1\0")?;
    let set: VtableOverrideFn = unsafe { core::mem::transmute(address) };
    let previous = unsafe { set(kind, index, space, function as u64) };
    (previous != 0).then_some(previous as usize)
}

type CommonVtableOverride2Fn = unsafe extern "C" fn(i32, u32, u32, usize, usize) -> u32;
static COMMON_VTABLE_OVERRIDE_2_FN: AtomicUsize = AtomicUsize::new(0);

fn set_common_vtable_entry_followed(
    kind: i32,
    slot: u32,
    space: u32,
    function: *const (),
    original: &AtomicUsize,
) -> Option<bool> {
    let address = resolve(
        &COMMON_VTABLE_OVERRIDE_2_FN,
        b"clone_engine_common_vtable_override_v2\0",
    )?;
    let set: CommonVtableOverride2Fn = unsafe { core::mem::transmute(address) };
    let taken = unsafe {
        set(
            kind,
            slot,
            space,
            function as usize,
            original as *const AtomicUsize as usize,
        )
    };
    Some(taken != 0)
}

type CopyCommonOverrideFn = unsafe extern "C" fn(i32, u32, usize, usize) -> u32;
static COPY_COMMON_OVERRIDE_FN: AtomicUsize = AtomicUsize::new(0);

fn set_copy_common_vtable_entry_followed(
    kind: i32,
    slot: u32,
    function: *const (),
    original: &AtomicUsize,
) -> Option<bool> {
    let address = resolve(
        &COPY_COMMON_OVERRIDE_FN,
        b"clone_engine_copy_common_vtable_override_v2\0",
    )?;
    let set: CopyCommonOverrideFn = unsafe { core::mem::transmute(address) };
    let taken = unsafe {
        set(
            kind,
            slot,
            function as usize,
            original as *const AtomicUsize as usize,
        )
    };
    Some(taken != 0)
}

fn set_vtable_entry_followed(
    kind: i32,
    index: u32,
    space: u32,
    function: *const (),
    original: &AtomicUsize,
) -> Option<bool> {
    let address = resolve(&VTABLE_OVERRIDE_2_FN, b"clone_engine_vtable_override_v2\0")?;
    let set: VtableOverride2Fn = unsafe { core::mem::transmute(address) };
    let taken = unsafe { set(kind, index, space, function as u64, original as *const AtomicUsize as u64) };
    Some(taken != 0)
}

pub fn item_virtual_function(kind: i32, index: u32, get_ptr: bool) -> usize {
    let Some(address) = resolve(&VTABLE_ENTRY_FN, b"clone_engine_vtable_entry_v1\0") else {
        return 0;
    };
    let function: VtableEntryFn = unsafe { core::mem::transmute(address) };
    unsafe { function(kind, index, SPACE_ITEM, u32::from(get_ptr)) as usize }
}

impl Item {
    pub fn vtable_entry(&self, index: u32) -> Option<u64> {
        let kind = self.kind().get()?;
        let offset = item_virtual_function(kind, index, false);
        (offset != 0).then_some(offset as u64)
    }

    pub fn set_vtable_entry(&self, index: u32, function: *const ()) -> Option<usize> {
        set_vtable_entry_in(self.kind().get()?, index, SPACE_ITEM, function)
    }

    pub fn set_common_vtable_entry(&self, slot: u32, function: *const ()) -> Option<usize> {
        set_common_vtable_entry_in(self.kind().get()?, slot, SPACE_ITEM, function)
    }
}

impl Fighter {
    pub fn vtable_entry(&self, index: u32) -> Option<u64> {
        let kind = self.kind().get()?;
        let offset = get_agent_virtual_function(kind, index as usize, false, false);
        (offset != 0).then_some(offset as u64)
    }

    pub fn set_vtable_entry(&self, index: u32, function: *const ()) -> Option<usize> {
        set_vtable_entry(self.kind().get()?, index, false, function)
    }

    pub fn set_common_vtable_entry(&self, slot: u32, function: *const ()) -> Option<usize> {
        set_common_vtable_entry(self.kind().get()?, slot, function)
    }
}

impl Fighter {
    pub fn set_copy_common_vtable_entry(
        &self,
        slot: u32,
        function: *const (),
        original: &'static AtomicUsize,
    ) -> bool {
        let Some(kind) = self.kind().get() else {
            return false;
        };
        set_copy_common_vtable_entry_followed(kind, slot, function, original).unwrap_or(false)
    }
}

impl Article {
    pub fn vtable_entry(&self, index: u32) -> Option<u64> {
        let kind = self.weapon_kind()?;
        let offset = get_agent_virtual_function(kind, index as usize, true, false);
        (offset != 0).then_some(offset as u64)
    }

    pub fn set_vtable_entry(&self, index: u32, function: *const ()) -> Option<usize> {
        set_vtable_entry(self.weapon_kind()?, index, true, function)
    }

    pub fn set_common_vtable_entry(&self, slot: u32, function: *const ()) -> Option<usize> {
        set_common_vtable_entry_in(self.weapon_kind()?, slot, SPACE_WEAPON, function)
    }
}

pub struct Override {
    site: Site,
    function: *const (),
    original: AtomicUsize,
    name: &'static str,
}

unsafe impl Sync for Override {}

impl Override {
    pub const fn new(site: Site, function: *const (), name: &'static str) -> Self {
        Self {
            site,
            function,
            original: AtomicUsize::new(0),
            name,
        }
    }

    pub fn original(&self) -> usize {
        self.original.load(Ordering::Acquire)
    }

    pub fn install(&self) -> Result<(), Error> {
        let (kind, index, space, what) = match &self.site {
            Site::FighterCommonSlot(fighter, index) => (
                fighter.kind().get(),
                *index,
                SPACE_FIGHTER,
                format!("{}'s common vtable slot {index}", fighter.identity),
            ),
            Site::CopyCommonSlot(fighter, index) => (
                fighter.kind().get(),
                *index,
                SPACE_FIGHTER,
                format!(
                    "{}'s copy common vtable slot {index}, which runs on Kirby",
                    fighter.identity
                ),
            ),
            Site::WeaponCommonSlot(fighter, article, index) => (
                fighter.article(article).weapon_kind(),
                *index,
                SPACE_WEAPON,
                format!(
                    "{}'s article {article} common vtable slot {index}",
                    fighter.identity
                ),
            ),
            Site::ItemCommonSlot(item, index) => (
                item.kind().get(),
                *index,
                SPACE_ITEM,
                format!("item {}'s common vtable slot {index}", item.identity),
            ),
            Site::FighterSlot(fighter, index) => (
                fighter.kind().get(),
                *index,
                SPACE_FIGHTER,
                format!("{}'s vtable slot {index}", fighter.identity),
            ),
            Site::WeaponSlot(fighter, article, index) => (
                fighter.article(article).weapon_kind(),
                *index,
                SPACE_WEAPON,
                format!("{}'s article {article} vtable slot {index}", fighter.identity),
            ),
            Site::ItemSlot(item, index) => (
                item.kind().get(),
                *index,
                SPACE_ITEM,
                format!("item {}'s vtable slot {index}", item.identity),
            ),
            Site::Offset(offset) => {
                elog!(
                    "[clone_engine] {}: an address ({offset:#x}) is not a vtable slot; use Hook for it",
                    self.name
                );
                return Err(Error::InvalidName);
            }
        };
        let Some(kind) = kind else {
            elog!(
                "[clone_engine] {}: {what} has no kind yet; register the fighter (and its articles) before install_hooks!",
                self.name
            );
            return Err(Error::EngineUnavailable);
        };
        if let Site::CopyCommonSlot(..) = self.site {
            match set_copy_common_vtable_entry_followed(kind, index, self.function, &self.original)
            {
                Some(true) => return Ok(()),
                Some(false) => {
                    elog!("[clone_engine] {}: {what} was refused", self.name);
                    return Err(Error::InvalidName);
                }
                None => return Err(Error::EngineUnavailable),
            }
        }
        let common = matches!(
            self.site,
            Site::FighterCommonSlot(..) | Site::WeaponCommonSlot(..) | Site::ItemCommonSlot(..)
        );
        let taken = if common {
            set_common_vtable_entry_followed(kind, index, space, self.function, &self.original)
        } else {
            set_vtable_entry_followed(kind, index, space, self.function, &self.original)
        };
        match taken {
            Some(true) => return Ok(()),
            Some(false) => {
                elog!(
                    "[clone_engine] {}: {what} (kind {kind}) was not taken; the slot is out of range or the kind has no base",
                    self.name
                );
                return Err(Error::EngineUnavailable);
            }
            None => {}
        }
        let Some(previous) = set_vtable_entry_in(kind, index, space, self.function) else {
            elog!(
                "[clone_engine] {}: {what} (kind {kind}) was not taken; the engine has no clone_engine_vtable_override_v1, the slot is out of range or the base's vtable is not readable yet",
                self.name
            );
            return Err(Error::EngineUnavailable);
        };
        self.original.store(previous, Ordering::Release);
        Ok(())
    }
}

pub enum Site {
    Offset(u64),
    FighterSlot(&'static Fighter, u32),
    WeaponSlot(&'static Fighter, &'static str, u32),
    ItemSlot(&'static Item, u32),
    FighterCommonSlot(&'static Fighter, u32),
    CopyCommonSlot(&'static Fighter, u32),
    WeaponCommonSlot(&'static Fighter, &'static str, u32),
    ItemCommonSlot(&'static Item, u32),
}

pub struct Hook {
    site: Site,
    resolved: AtomicU64,
    argument_count: u32,
    expect: Option<[u32; 4]>,
    callback: HookFn,
    name: &'static str,
}

impl Hook {
    pub const fn new(
        offset: u64,
        argument_count: u32,
        expect: Option<[u32; 4]>,
        callback: HookFn,
        name: &'static str,
    ) -> Self {
        Self::at(Site::Offset(offset), argument_count, expect, callback, name)
    }

    pub const fn at(
        site: Site,
        argument_count: u32,
        expect: Option<[u32; 4]>,
        callback: HookFn,
        name: &'static str,
    ) -> Self {
        Self {
            site,
            resolved: AtomicU64::new(0),
            argument_count,
            expect,
            callback,
            name,
        }
    }

    pub fn offset(&self) -> u64 {
        let known = self.resolved.load(Ordering::Acquire);
        if known != 0 {
            return known;
        }
        let found = match &self.site {
            Site::Offset(offset) => *offset,
            Site::FighterSlot(fighter, index) => fighter.vtable_entry(*index).unwrap_or(0),
            Site::WeaponSlot(fighter, article, index) => {
                fighter.article(article).vtable_entry(*index).unwrap_or(0)
            }
            Site::ItemSlot(item, index) => item.vtable_entry(*index).unwrap_or(0),
            Site::FighterCommonSlot(..)
            | Site::WeaponCommonSlot(..)
            | Site::ItemCommonSlot(..)
            | Site::CopyCommonSlot(..) => 0,
        };
        if found != 0 {
            self.resolved.store(found, Ordering::Release);
        }
        found
    }

    fn describe(&self) -> String {
        match &self.site {
            Site::Offset(offset) => format!("{offset:#x}"),
            Site::FighterSlot(fighter, index) => format!("{}'s vtable slot {index}", fighter.identity),
            Site::WeaponSlot(fighter, article, index) => {
                format!("{}'s article {article} vtable slot {index}", fighter.identity)
            }
            Site::ItemSlot(item, index) => format!("item {}'s vtable slot {index}", item.identity),
            Site::FighterCommonSlot(fighter, index) => {
                format!("{}'s common vtable slot {index}", fighter.identity)
            }
            Site::WeaponCommonSlot(fighter, article, index) => format!(
                "{}'s article {article} common vtable slot {index}",
                fighter.identity
            ),
            Site::ItemCommonSlot(item, index) => {
                format!("item {}'s common vtable slot {index}", item.identity)
            }
            Site::CopyCommonSlot(fighter, index) => format!(
                "{}'s copy common vtable slot {index}",
                fighter.identity
            ),
        }
    }

    fn live_words(&self, offset: u64) -> Option<[u32; 4]> {
        let text = unsafe { skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) } as usize;
        if text == 0 {
            return None;
        }
        let site = text.checked_add(usize::try_from(offset).ok()?)? as *const u32;
        let mut words = [0u32; 4];
        for (index, word) in words.iter_mut().enumerate() {
            *word = unsafe { core::ptr::read_volatile(site.add(index)) };
        }
        Some(words)
    }

    pub fn install(&self) -> Result<(), Error> {
        let offset = self.offset();
        if offset == 0 {
            elog!(
                "[clone_engine] hook {}: {} could not be resolved; the fighter is not registered yet or the engine has no vtable export",
                self.name,
                self.describe()
            );
            return Err(Error::EngineUnavailable);
        }
        let words = match self.expect {
            Some(expect) => expect,
            None => {
                let live = self.live_words(offset).ok_or(Error::EngineUnavailable)?;
                let shared = crate::shared_hook_words(offset);
                let Some(words) = dionysus_core::hook_site::entry_words(&live, shared) else {
                    elog!(
                        "[clone_engine] hook {} at {:#x} ({}): the site is not an untouched function entry ({:#010x} {:#010x}) and no other pack shares it through the engine; another plugin hooked it exclusively, or the offset is wrong",
                        self.name,
                        offset,
                        self.describe(),
                        live[0],
                        live[1]
                    );
                    return Err(Error::Engine(crate::ERROR_HOOK_PREFLIGHT));
                };
                words
            }
        };
        let registration =
            SharedHookRegistrationV1::new(offset, words, self.argument_count, self.callback);
        unsafe { shared_hook_checked(&registration) }
    }
}

#[macro_export]
macro_rules! install_hooks {
    ($($hook:expr),* $(,)?) => {
        $(
            if let Err(error) = $hook.install() {
                $crate::elog!("[clone_engine] hook {} was not installed: {error:?}", stringify!($hook));
            }
        )*
    };
}

#[macro_export]
macro_rules! fighter {
    ($name:ident, $identity:expr) => {
        pub static $name: $crate::v2::Fighter = $crate::v2::Fighter::new($identity);
        #[doc(hidden)]
        pub static __CLONE_ENGINE_FIGHTER: &$crate::v2::Fighter = &$name;
    };
}

#[macro_export]
macro_rules! item {
    ($name:ident, $identity:expr) => {
        pub static $name: $crate::v2::Item = $crate::v2::Item::new($identity);
    };
}

pub struct Item {
    identity: &'static str,
    kind: AtomicI32,
}

impl Item {
    pub const fn new(identity: &'static str) -> Self {
        Self {
            identity,
            kind: AtomicI32::new(UNRESOLVED),
        }
    }

    pub fn identity(&self) -> &'static str {
        self.identity
    }

    pub fn kind(&self) -> Kind {
        let cached = self.kind.load(Ordering::Acquire);
        if cached != UNRESOLVED {
            return Kind(Some(cached));
        }
        let found = crate::item_kind_for_identity(self.identity);
        if let Some(kind) = found {
            self.kind.store(kind, Ordering::Release);
        }
        Kind(found)
    }

    pub fn register(&self, manifest: ItemManifest) -> Result<i32, Error> {
        if manifest.resource_name() != self.identity {
            elog!(
                "[clone_engine] item {}: the manifest is for {:?}; the identity and the resource name must agree",
                self.identity,
                manifest.resource_name()
            );
            return Err(Error::InvalidName);
        }
        if let Some(kind) = self.kind().get() {
            elog!(
                "[clone_engine] item {}: already registered as kind {kind:#x} (an item.toml in the pack?); the manifest is ignored",
                self.identity
            );
            return Ok(kind);
        }
        let kind = register_item_manifest(self.identity, &manifest.to_toml())?;
        self.kind.store(kind, Ordering::Release);
        Ok(kind)
    }

    pub fn ready(&self) -> bool {
        self.kind().is_some()
    }

    pub fn is<T: AsBoma>(&self, object: T) -> bool {
        let Some(kind) = self.kind().get() else {
            return false;
        };
        let boma = object.as_boma();
        !boma.is_null() && unsafe { crate::item_kind_from_boma(boma) } == kind
    }

    pub fn status_raw(&self, line: crate::ItemStatusLine, status_name: &str, function: *const ()) -> bool {
        let Some(kind) = self.kind().get() else {
            elog!("[clone_engine] item {}: not registered; add it to item.toml", self.identity);
            return false;
        };
        match crate::item_status_named(kind, line, status_name, function) {
            Ok(()) => true,
            Err(error) => {
                elog!(
                    "[clone_engine] item {}: status {status_name} {line:?} refused: {error:?}",
                    self.identity
                );
                false
            }
        }
    }

    #[cfg(feature = "smash")]
    pub fn status(
        &self,
        line: crate::ItemStatusLine,
        status_name: &str,
        function: unsafe extern "C" fn(&mut smash::lua2cpp::L2CFighterCommon) -> smash::lib::L2CValue,
    ) -> bool {
        self.status_raw(line, status_name, function as *const ())
    }

    pub fn owner_param(&self, owner: &str, field: &str) -> OwnerParam<'_> {
        OwnerParam {
            item: self,
            owner: owner.to_string(),
            field: field.to_string(),
        }
    }
}

type FighterKindByNameFn = unsafe extern "C" fn(*const std::os::raw::c_char) -> i32;
static FIGHTER_KIND_BY_NAME_FN: AtomicUsize = AtomicUsize::new(0);

pub fn fighter_kind_by_name(name: &str) -> Option<i32> {
    let name = CString::new(name).ok()?;
    let address = resolve(&FIGHTER_KIND_BY_NAME_FN, b"clone_engine_fighter_kind_by_name_v1\0")?;
    let function: FighterKindByNameFn = unsafe { core::mem::transmute(address) };
    let kind = unsafe { function(name.as_ptr()) };
    (kind >= 0).then_some(kind)
}

pub struct OwnerParam<'a> {
    item: &'a Item,
    owner: String,
    field: String,
}

impl OwnerParam<'_> {
    fn resolve(&self) -> Option<(i32, i32, &'static dionysus_core::owner_param_words::Word)> {
        let item_kind = self.item.kind().get()?;
        let owner_kind = fighter_kind_by_name(&self.owner)?;
        let word = dionysus_core::owner_param_words::words_of(owner_kind)
            .iter()
            .find(|word| word.path() == self.field || word.field_name() == self.field)?;
        Some((item_kind, owner_kind, word))
    }

    pub fn set(self, value: f64) -> bool {
        let Some((item_kind, owner_kind, word)) = self.resolve() else {
            elog!(
                "[clone_engine] item {}: no owner param {}/{}",
                self.item.identity,
                self.owner,
                self.field
            );
            return false;
        };
        let result = match word.kind {
            dionysus_core::owner_param_words::WordKind::F32 => {
                crate::item_owner_param_set_f32(item_kind, owner_kind, u32::from(word.offset), value as f32)
            }
            _ => crate::item_owner_param_set_i32(item_kind, owner_kind, u32::from(word.offset), value as i32),
        };
        match result {
            Ok(()) => true,
            Err(error) => {
                elog!(
                    "[clone_engine] item {}: owner param {}/{} refused: {error:?}",
                    self.item.identity,
                    self.owner,
                    self.field
                );
                false
            }
        }
    }
}
