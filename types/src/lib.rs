//! Structures for the various Inko types.
#![allow(clippy::new_without_default)]
#![allow(clippy::len_without_is_empty)]
#![allow(clippy::too_many_arguments)]

#[cfg(test)]
pub mod test;

pub mod check;
pub mod either;
pub mod format;
pub mod module_name;
pub mod resolve;
pub mod specialize;

use crate::module_name::ModuleName;
use crate::resolve::TypeResolver;
use indexmap::IndexMap;
use location::Location;
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

// The IDs of these built-in types must match the order of the fields in the
// State type.
pub const STRING_ID: u32 = 0;
pub const BYTE_ARRAY_ID: u32 = 1;
pub const INT_ID: u32 = 2;
pub const FLOAT_ID: u32 = 3;
pub const BOOL_ID: u32 = 4;
pub const NIL_ID: u32 = 5;

const TUPLE1_ID: u32 = 6;
const TUPLE2_ID: u32 = 7;
const TUPLE3_ID: u32 = 8;
const TUPLE4_ID: u32 = 9;
const TUPLE5_ID: u32 = 10;
const TUPLE6_ID: u32 = 11;
const TUPLE7_ID: u32 = 12;
const TUPLE8_ID: u32 = 13;
const ARRAY_ID: u32 = 14;
const CHECKED_INT_RESULT_ID: u32 = 15;

pub const FIRST_USER_CLASS_ID: u32 = CHECKED_INT_RESULT_ID + 1;

/// The default module ID to assign to builtin types.
///
/// This ID is corrected using a `builtin class` definition.
const DEFAULT_BUILTIN_MODULE_ID: u32 = 0;

const INT_NAME: &str = "Int";
const FLOAT_NAME: &str = "Float";
const STRING_NAME: &str = "String";
const ARRAY_NAME: &str = "Array";
const BOOL_NAME: &str = "Bool";
const NIL_NAME: &str = "Nil";
const BYTE_ARRAY_NAME: &str = "ByteArray";
const TUPLE1_NAME: &str = "Tuple1";
const TUPLE2_NAME: &str = "Tuple2";
const TUPLE3_NAME: &str = "Tuple3";
const TUPLE4_NAME: &str = "Tuple4";
const TUPLE5_NAME: &str = "Tuple5";
const TUPLE6_NAME: &str = "Tuple6";
const TUPLE7_NAME: &str = "Tuple7";
const TUPLE8_NAME: &str = "Tuple8";
const CHECKED_INT_RESULT_NAME: &str = "CheckedIntResult";

pub const STRING_MODULE: &str = "std.string";
pub const TO_STRING_TRAIT: &str = "ToString";
pub const TO_STRING_METHOD: &str = "to_string";
pub const CALL_METHOD: &str = "call";
pub const EQ_METHOD: &str = "==";
pub const MAIN_CLASS: &str = "Main";
pub const MAIN_METHOD: &str = "main";
pub const DROP_MODULE: &str = "std.drop";
pub const DROP_TRAIT: &str = "Drop";
pub const DROP_METHOD: &str = "drop";
pub const DROPPER_METHOD: &str = "$dropper";
pub const ASYNC_DROPPER_METHOD: &str = "$async_dropper";
pub const OPTION_MODULE: &str = "std.option";
pub const OPTION_CLASS: &str = "Option";
pub const RESULT_MODULE: &str = "std.result";
pub const RESULT_CLASS: &str = "Result";
pub const OPTION_SOME: &str = "Some";
pub const OPTION_NONE: &str = "None";
pub const RESULT_OK: &str = "Ok";
pub const RESULT_ERROR: &str = "Error";
pub const ARRAY_WITH_CAPACITY: &str = "with_capacity";
pub const ARRAY_PUSH: &str = "push";
pub const ARRAY_INTERNAL_NAME: &str = "$Array";

/// The name of the pseudo field used to deference a pointer.
pub const DEREF_POINTER_FIELD: &str = "0";

pub const ENUM_TAG_FIELD: &str = "tag";
pub const ENUM_TAG_INDEX: usize = 0;

/// The maximum number of enum constructors that can be defined in a single
/// class.
pub const CONSTRUCTORS_LIMIT: usize = u16::MAX as usize;

/// The maximum number of fields a class can define.
pub const FIELDS_LIMIT: usize = u8::MAX as usize;

/// The maximum number of values that can be stored in an array literal.
pub const ARRAY_LIMIT: usize = u16::MAX as usize;

/// The maximum number of methods supported.
///
/// This is one less than the u32 maximum such that we can use `u32::MAX` as a
/// sentinel value in various places.
const METHODS_LIMIT: usize = (u32::MAX - 1) as usize;

/// When a symbol is using this name, the source module should be imported
/// instead of the symbol.
pub const IMPORT_MODULE_ITSELF_NAME: &str = "self";

/// The requirement of a type inference placeholder.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum PlaceholderRequirement {
    None,
    Owned(TypeParameterId),
    Any(TypeParameterId),
}

/// A type inference placeholder.
///
/// A type placeholder reprents a value of which the exact type isn't
/// immediately known, and is to be inferred based on how the value is used.
/// Take this code for example:
///
///     let vals = []
///
/// While we know that `vals` is an array, we don't know the type of the values
/// in the array. In this case we use a type placeholder, meaning that `vals` is
/// of type `Array[V₁]` where V₁ is a type placeholder.
///
/// At some point we may push a value into the array, for example:
///
///     vals.push(42)
///
/// In this case V₁ is assigned to `Int`, and we end up with `vals` inferred as
/// `Array[Int]`.
///
/// The concept of type placeholder is taken from the Hindley-Milner type
/// system.
pub struct TypePlaceholder {
    /// The value assigned to this placeholder.
    ///
    /// This is wrapped in a Cell so we don't need a mutable borrow to the
    /// Database when updating a placeholder. This in turn is needed because
    /// type-checking functions expect/depend on an immutable database, and
    /// can't work with a mutable one (e.g. due to having to borrow multiple
    /// fields).
    value: Cell<TypeRef>,

    /// The type parameter requirement that must be met before a type is
    /// compatible with this placeholder.
    required: Option<TypeParameterId>,
}

impl TypePlaceholder {
    fn alloc(
        db: &mut Database,
        required: Option<TypeParameterId>,
    ) -> TypePlaceholderId {
        assert!(db.type_placeholders.len() < u32::MAX as usize);

        let id = db.type_placeholders.len() as u32;
        let typ =
            TypePlaceholder { value: Cell::new(TypeRef::Unknown), required };

        db.type_placeholders.push(typ);
        TypePlaceholderId { id, ownership: Ownership::Any }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
enum Ownership {
    Any,
    Owned,
    Uni,
    Ref,
    Mut,
    UniRef,
    UniMut,
    Pointer,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypePlaceholderId {
    id: u32,

    /// The ownership values must have before they can be assigned to the
    /// placeholder.
    ///
    /// This is stored in the ID/reference as in various instances type
    /// placeholders are created ahead of time, at which point we do not yet
    /// know the desired ownership. In addition, based on how a type parameter
    /// is used its ownership may be inferred after it's created.
    ///
    /// By storing this in the ID we can adjust it accordingly where necessary.
    /// When resolving placeholder values, their ownership is adjusted according
    /// to the ownership of the placeholder.
    ownership: Ownership,
}

impl TypePlaceholderId {
    fn with_ownership(self, ownership: Ownership) -> TypePlaceholderId {
        TypePlaceholderId { id: self.id, ownership }
    }

    fn as_owned(self) -> TypePlaceholderId {
        self.with_ownership(Ownership::Owned)
    }

    fn as_uni(self) -> TypePlaceholderId {
        self.with_ownership(Ownership::Uni)
    }

    fn as_ref(self) -> TypePlaceholderId {
        self.with_ownership(Ownership::Ref)
    }

    fn as_pointer(self) -> TypePlaceholderId {
        self.with_ownership(Ownership::Pointer)
    }

    fn as_mut(self) -> TypePlaceholderId {
        self.with_ownership(Ownership::Mut)
    }

    fn as_uni_ref(self) -> TypePlaceholderId {
        self.with_ownership(Ownership::UniRef)
    }

    fn as_uni_mut(self) -> TypePlaceholderId {
        self.with_ownership(Ownership::UniMut)
    }

    pub fn value(self, db: &Database) -> Option<TypeRef> {
        // Chains of type variables are very rare in practise, but they _can_
        // occur and thus must be handled. Because they are so rare and unlikely
        // to be more than 2-3 levels deep, we just use recursion here instead
        // of a loop.
        let typ = self.get(db).value.get();

        match typ {
            TypeRef::Placeholder(id) => id.value(db),
            TypeRef::Unknown => None,
            _ => {
                let res = match self.ownership {
                    Ownership::Any => typ,
                    Ownership::Owned => typ.as_owned(db),
                    Ownership::Uni => typ.as_uni(db),
                    Ownership::Ref => typ.as_ref(db),
                    Ownership::Mut => typ.force_as_mut(db),
                    Ownership::UniRef => typ.as_uni_ref(db),
                    Ownership::UniMut => typ.force_as_uni_mut(db),
                    Ownership::Pointer => typ.as_pointer(db),
                };

                Some(res)
            }
        }
    }

    fn required(self, db: &Database) -> Option<TypeParameterId> {
        self.get(db).required
    }

    /// Assigns the placeholder the given value, relying on interior mutability.
    ///
    /// This method exists so we can assign a placeholder a type during type
    /// checking. We can't use a `&mut Database` there as doing so results in
    /// borrowing errors.
    pub(crate) fn assign_internal(self, db: &Database, value: TypeRef) {
        // Assigning placeholders to themselves isn't useful and results in
        // resolve() getting stuck.
        if let TypeRef::Placeholder(id) = value {
            if id.id == self.id {
                return;
            }
        }

        self.get(db).value.set(value);
    }

    /// Assigns the placeholder the given value.
    ///
    /// This method differs from `assign_internal` in that it requires a
    /// `&mut Database`. This is meant to be used outside of this crate and
    /// ensures one can't concurrently modify a `TypePlaceholder`.
    pub fn assign(self, db: &mut Database, value: TypeRef) {
        self.assign_internal(db, value);
    }

    pub(crate) fn has_ownership(self) -> bool {
        !matches!(self.ownership, Ownership::Any)
    }

    fn get(self, db: &Database) -> &TypePlaceholder {
        &db.type_placeholders[self.id as usize]
    }
}

// TypePlaceholder uses interior mutability for storing the type assigned to a
// placeholder, thus making it `!Sync` by default. This prevents us from
// using a `&Database` in multiple threads, even if they never mutate a
// `TypePlaceholder`.
//
// To make this possible and safe, only code in this crate can assign types
// through a `&Database`, while code in other crates must go through
// `TypePlaceholder::assign()`, which requires a `&mut Database`.
unsafe impl Sync for TypePlaceholder {}

/// A type parameter for a method or class.
#[derive(Clone)]
pub struct TypeParameter {
    /// The name of the type parameter.
    name: String,

    /// The traits that must be implemented before a type can be assigned to
    /// this type parameter.
    requirements: Vec<TraitInstance>,

    /// If mutable references to this type parameter are allowed.
    mutable: bool,

    /// If types assigned to this parameter must be allocated on the stack.
    stack: bool,

    /// The ID of the original type parameter in case the current one is a
    /// parameter introduced through additional type bounds.
    original: Option<TypeParameterId>,
}

impl TypeParameter {
    pub fn alloc(db: &mut Database, name: String) -> TypeParameterId {
        TypeParameter::add(db, TypeParameter::new(name))
    }

    fn add(db: &mut Database, parameter: TypeParameter) -> TypeParameterId {
        let id = db.type_parameters.len();

        db.type_parameters.push(parameter);
        TypeParameterId(id)
    }

    fn new(name: String) -> Self {
        Self {
            name,
            requirements: Vec::new(),
            mutable: false,
            stack: false,
            original: None,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct TypeParameterId(pub usize);

impl TypeParameterId {
    pub fn name(self, db: &Database) -> &String {
        &self.get(db).name
    }

    pub fn add_requirements(
        self,
        db: &mut Database,
        mut requirements: Vec<TraitInstance>,
    ) {
        self.get_mut(db).requirements.append(&mut requirements);
    }

    pub fn requirements(self, db: &Database) -> Vec<TraitInstance> {
        self.get(db).requirements.clone()
    }

    pub fn method(self, db: &Database, name: &str) -> Option<MethodId> {
        let typ = self.get(db);

        for &req in &typ.requirements {
            if let Some(m) = req.method(db, name) {
                return Some(m);
            }
        }

        None
    }

    pub fn set_original(self, db: &mut Database, parameter: TypeParameterId) {
        self.get_mut(db).original = Some(parameter);
    }

    pub fn original(self, db: &Database) -> Option<TypeParameterId> {
        self.get(db).original
    }

    pub fn set_mutable(self, db: &mut Database) {
        self.get_mut(db).mutable = true;
    }

    pub fn is_mutable(self, db: &Database) -> bool {
        self.get(db).mutable
    }

    pub fn set_stack_allocated(self, db: &mut Database) {
        self.get_mut(db).stack = true;
    }

    pub fn is_stack_allocated(self, db: &Database) -> bool {
        self.get(db).stack
    }

    pub fn as_immutable(self, db: &mut Database) -> TypeParameterId {
        let mut copy = self.get(db).clone();

        copy.mutable = false;
        TypeParameter::add(db, copy)
    }

    pub fn clone_for_bound(self, db: &mut Database) -> TypeParameterId {
        let mut copy = self.get(db).clone();

        copy.original = Some(self);
        TypeParameter::add(db, copy)
    }

    pub(crate) fn has_requirements(self, db: &Database) -> bool {
        !self.get(db).requirements.is_empty()
    }

    fn get(self, db: &Database) -> &TypeParameter {
        &db.type_parameters[self.0]
    }

    fn get_mut(self, db: &mut Database) -> &mut TypeParameter {
        &mut db.type_parameters[self.0]
    }

    fn as_rigid(self) -> TypeRef {
        TypeRef::Any(TypeId::RigidTypeParameter(self))
    }
}

/// Type parameters and the types assigned to them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeArguments {
    /// We use a HashMap as parameters can be assigned in any order, and some
    /// may not be assigned at all.
    mapping: HashMap<TypeParameterId, TypeRef>,
}

impl TypeArguments {
    pub fn for_class(db: &Database, instance: ClassInstance) -> TypeArguments {
        if instance.instance_of().is_generic(db) {
            instance.type_arguments(db).unwrap().clone()
        } else {
            TypeArguments::new()
        }
    }

    pub fn for_trait(db: &Database, instance: TraitInstance) -> TypeArguments {
        if instance.instance_of().is_generic(db) {
            instance.type_arguments(db).unwrap().clone()
        } else {
            TypeArguments::new()
        }
    }

    pub fn new() -> Self {
        Self { mapping: HashMap::default() }
    }

    pub fn assign(&mut self, parameter: TypeParameterId, value: TypeRef) {
        self.mapping.insert(parameter, value);
    }

    pub fn get(&self, parameter: TypeParameterId) -> Option<TypeRef> {
        self.mapping.get(&parameter).cloned()
    }

    pub fn get_recursive(
        &self,
        db: &Database,
        parameter: TypeParameterId,
    ) -> Option<TypeRef> {
        let mut found = self.get(parameter);

        while let Some(typ) = found {
            let id = if let Some(id) = typ.as_type_parameter(db) {
                id
            } else {
                return Some(typ);
            };

            match self.get(id) {
                Some(new) if new == typ => return Some(typ),
                Some(new) => found = Some(new),
                None => return Some(typ),
            }
        }

        None
    }

    pub fn pairs(&self) -> Vec<(TypeParameterId, TypeRef)> {
        self.mapping.iter().map(|(&a, &b)| (a, b)).collect()
    }

    pub fn keys(&self) -> impl Iterator<Item = &TypeParameterId> {
        self.mapping.keys()
    }

    pub fn copy_into(&self, other: &mut Self) {
        for (&key, &value) in &self.mapping {
            other.assign(key, value);
        }
    }

    pub fn move_into(self, other: &mut Self) {
        for (key, value) in self.mapping {
            other.assign(key, value);
        }
    }

    pub fn copy_assigned_into(
        &self,
        parameters: Vec<TypeParameterId>,
        target: &mut Self,
    ) {
        for param in parameters {
            if let Some(value) = self.get(param) {
                target.assign(param, value);
            }
        }
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut TypeRef> {
        self.mapping.values_mut()
    }

    pub fn is_empty(&self) -> bool {
        self.mapping.is_empty()
    }

    pub fn iter(
        &self,
    ) -> std::collections::hash_map::Iter<TypeParameterId, TypeRef> {
        self.mapping.iter()
    }
}

/// A type that maps/interns type arguments, such that structurually different
/// but semantically equivalent type arguments all map to the same type arguments
/// ID.
///
/// As part of type specialization we want to specialize over specific types,
/// such as when using `Shape::Stack`. Since type argument IDs differ for
/// different occurrences of the same generic type instance, we need a way to
/// map those to a common type arguments ID. If we don't do this, we may end up
/// specializing the same type many times.
pub struct InternedTypeArguments {
    /// A cache that maps the raw class instances to their interned type
    /// arguments ID.
    ///
    /// This cache is used to avoid the more expensive key generation process
    /// when comparing the exact same type many times.
    cache: HashMap<ClassInstance, u32>,

    /// A mapping of the flattened type IDs from a class instance to the common
    /// type arguments ID.
    ///
    /// For ClassInstance and TraitInstance types, the TypeId is stripped of its
    /// TypeArguments ID such that it's consistent when hashed.
    mapping: HashMap<Vec<TypeId>, u32>,
}

impl InternedTypeArguments {
    pub fn new() -> InternedTypeArguments {
        InternedTypeArguments { cache: HashMap::new(), mapping: HashMap::new() }
    }

    pub fn intern(&mut self, db: &Database, instance: ClassInstance) -> u32 {
        // The cache is used such that if we use the exact same type N times, we
        // only perform the more expensive type walking once.
        if let Some(&id) = self.cache.get(&instance) {
            return id;
        }

        let mut key = Vec::new();
        let mut stack = vec![TypeId::ClassInstance(instance)];

        // The order of the values in the key doesn't matter, as long as it's
        // consistent.
        while let Some(tid) = stack.pop() {
            let (val, args) = match tid {
                TypeId::ClassInstance(i) if i.instance_of().is_generic(db) => (
                    TypeId::ClassInstance(ClassInstance::new(i.instance_of())),
                    i.type_arguments(db),
                ),
                TypeId::TraitInstance(i) if i.instance_of().is_generic(db) => (
                    TypeId::TraitInstance(TraitInstance::new(i.instance_of())),
                    i.type_arguments(db),
                ),
                _ => (tid, None),
            };

            if let Some(args) = args {
                for id in args.iter().flat_map(|(_, t)| t.type_id(db).ok()) {
                    stack.push(id);
                }
            }

            key.push(val);
        }

        let id = *self.mapping.entry(key).or_insert(instance.type_arguments);

        self.cache.insert(instance, id);
        id
    }
}

/// An Inko trait.
pub struct Trait {
    name: String,
    module: ModuleId,
    location: Location,
    documentation: String,
    implemented_by: Vec<ClassId>,
    visibility: Visibility,
    type_parameters: IndexMap<String, TypeParameterId>,
    required_traits: Vec<TraitInstance>,
    default_methods: IndexMap<String, MethodId>,
    required_methods: IndexMap<String, MethodId>,

    /// The type arguments inherited from any of the required traits.
    ///
    /// Traits may require generic traits, which in turn can require other
    /// generic traits, and so on. When we have an instance of trait `T`, we may
    /// end up calling a method from one of its ancestors. If that method
    /// returns a type parameter, we want to map it to the proper type. Consider
    /// this chain of types:
    ///
    ///     trait A[P1] {
    ///       fn foo -> P1
    ///     }
    ///
    ///     trait B[P2]: A[P2] {}
    ///     trait C[P3]: B[P3] {}
    ///
    /// Given an instance of `C[Int]`, `foo` should return `Int` as well, even
    /// though `P1` isn't explicitly assigned a value in `C[Int]`. Since walking
    /// the entire trait requirement chain every lookup is expensive, we store
    /// the inherited arguments. In the above example that means this structure
    /// contains the following mappings:
    ///
    ///     P2 -> P3
    ///     P1 -> P2
    ///
    /// Whenever we need to lookup type parameter assignments for an instance of
    /// `C`, we just copy this structure and use it for lookups and updates
    /// accordingly.
    ///
    /// Since most traits don't require many other traits, the overhead of this
    /// should be minimal, and less compared to walking requirement chains when
    /// performing lookups.
    inherited_type_arguments: TypeArguments,
}

impl Trait {
    pub fn alloc(
        db: &mut Database,
        name: String,
        visibility: Visibility,
        module: ModuleId,
        location: Location,
    ) -> TraitId {
        assert!(db.traits.len() < u32::MAX as usize);

        let id = db.traits.len() as u32;
        let trait_type = Trait::new(name, visibility, module, location);

        db.traits.push(trait_type);
        TraitId(id)
    }

    fn new(
        name: String,
        visibility: Visibility,
        module: ModuleId,
        location: Location,
    ) -> Self {
        Self {
            name,
            visibility,
            module,
            location,
            documentation: String::new(),
            implemented_by: Vec::new(),
            type_parameters: IndexMap::new(),
            required_traits: Vec::new(),
            default_methods: IndexMap::new(),
            required_methods: IndexMap::new(),
            inherited_type_arguments: TypeArguments::new(),
        }
    }

    fn is_generic(&self) -> bool {
        !self.type_parameters.is_empty()
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct TraitId(pub u32);

impl TraitId {
    pub fn name(self, db: &Database) -> &String {
        &self.get(db).name
    }

    pub fn type_parameters(self, db: &Database) -> Vec<TypeParameterId> {
        self.get(db).type_parameters.values().cloned().collect()
    }

    pub fn required_traits(self, db: &Database) -> Vec<TraitInstance> {
        self.get(db).required_traits.clone()
    }

    pub fn required_methods(self, db: &Database) -> Vec<MethodId> {
        self.get(db).required_methods.values().cloned().collect()
    }

    pub fn default_methods(self, db: &Database) -> Vec<MethodId> {
        self.get(db).default_methods.values().cloned().collect()
    }

    pub fn add_required_trait(
        self,
        db: &mut Database,
        requirement: TraitInstance,
    ) {
        let mut base =
            requirement.instance_of.get(db).inherited_type_arguments.clone();

        if requirement.instance_of.is_generic(db) {
            requirement.type_arguments(db).unwrap().copy_into(&mut base);
        }

        let self_typ = self.get_mut(db);

        base.move_into(&mut self_typ.inherited_type_arguments);
        self_typ.required_traits.push(requirement);
    }

    pub fn implemented_by(self, db: &Database) -> &Vec<ClassId> {
        &self.get(db).implemented_by
    }

    pub fn method_exists(self, db: &Database, name: &str) -> bool {
        self.get(db).default_methods.contains_key(name)
            || self.get(db).required_methods.contains_key(name)
    }

    pub fn method(self, db: &Database, name: &str) -> Option<MethodId> {
        let typ = self.get(db);

        if let Some(&id) = typ
            .default_methods
            .get(name)
            .or_else(|| typ.required_methods.get(name))
        {
            return Some(id);
        }

        for &req in &typ.required_traits {
            if let Some(id) = req.method(db, name) {
                return Some(id);
            }
        }

        None
    }

    pub fn add_default_method(
        self,
        db: &mut Database,
        name: String,
        method: MethodId,
    ) {
        self.get_mut(db).default_methods.insert(name, method);
    }

    pub fn add_required_method(
        self,
        db: &mut Database,
        name: String,
        method: MethodId,
    ) {
        self.get_mut(db).required_methods.insert(name, method);
    }

    pub fn is_generic(self, db: &Database) -> bool {
        self.get(db).is_generic()
    }

    pub fn number_of_type_parameters(&self, db: &Database) -> usize {
        self.get(db).type_parameters.len()
    }

    pub fn type_parameter_exists(&self, db: &Database, name: &str) -> bool {
        self.get(db).type_parameters.contains_key(name)
    }

    pub fn new_type_parameter(
        &self,
        db: &mut Database,
        name: String,
    ) -> TypeParameterId {
        let param = TypeParameter::alloc(db, name.clone());

        self.get_mut(db).type_parameters.insert(name, param);
        param
    }

    pub fn is_public(self, db: &Database) -> bool {
        self.get(db).visibility == Visibility::Public
    }

    pub fn is_private(self, db: &Database) -> bool {
        !self.is_public(db)
    }

    pub fn inherited_type_arguments(self, db: &Database) -> &TypeArguments {
        &self.get(db).inherited_type_arguments
    }

    pub fn location(self, db: &Database) -> Location {
        self.get(db).location
    }

    pub fn set_documentation(self, db: &mut Database, value: String) {
        self.get_mut(db).documentation = value;
    }

    pub fn documentation(self, db: &Database) -> &String {
        &self.get(db).documentation
    }

    fn named_type(self, db: &Database, name: &str) -> Option<Symbol> {
        self.get(db)
            .type_parameters
            .get(name)
            .map(|&id| Symbol::TypeParameter(id))
    }

    pub fn module(self, db: &Database) -> ModuleId {
        self.get(db).module
    }

    fn get(self, db: &Database) -> &Trait {
        &db.traits[self.0 as usize]
    }

    fn get_mut(self, db: &mut Database) -> &mut Trait {
        &mut db.traits[self.0 as usize]
    }
}

/// An instance of a trait, along with its type arguments in case the trait is
/// generic.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TraitInstance {
    /// The ID of the trait we're an instance of.
    instance_of: TraitId,

    /// The index to the TypeArguments structure that belongs to this trait
    /// instance.
    ///
    /// If the trait is a regular trait, this ID is always 0 and shouldn't be
    /// used.
    ///
    /// After type specialization takes place, this value shouldn't be used any
    /// more as specialized types won't have their type arguments set.
    type_arguments: u32,
}

impl TraitInstance {
    pub fn new(instance_of: TraitId) -> Self {
        Self { instance_of, type_arguments: 0 }
    }

    pub fn rigid(
        db: &mut Database,
        instance_of: TraitId,
        bounds: &TypeBounds,
    ) -> Self {
        if instance_of.is_generic(db) {
            let mut arguments = TypeArguments::new();

            for param in instance_of.type_parameters(db) {
                arguments.assign(
                    param,
                    bounds.get(param).unwrap_or(param).as_rigid(),
                );
            }

            Self::generic(db, instance_of, arguments)
        } else {
            Self::new(instance_of)
        }
    }

    pub fn generic(
        db: &mut Database,
        instance_of: TraitId,
        arguments: TypeArguments,
    ) -> Self {
        assert!(db.type_arguments.len() < u32::MAX as usize);

        let type_args_id = db.type_arguments.len() as u32;

        db.type_arguments.push(arguments);
        TraitInstance { instance_of, type_arguments: type_args_id }
    }

    pub fn instance_of(self) -> TraitId {
        self.instance_of
    }

    pub fn type_arguments(self, db: &Database) -> Option<&TypeArguments> {
        db.type_arguments.get(self.type_arguments as usize)
    }

    pub fn copy_new_arguments_from(
        self,
        db: &mut Database,
        from: &TypeArguments,
    ) {
        if !self.instance_of.is_generic(db) {
            return;
        }

        let params = self.instance_of.type_parameters(db);
        let targs = &mut db.type_arguments[self.type_arguments as usize];

        from.copy_assigned_into(params, targs);
    }

    pub fn copy_type_arguments_into(
        self,
        db: &Database,
        target: &mut TypeArguments,
    ) {
        if !self.instance_of.is_generic(db) {
            return;
        }

        self.type_arguments(db).unwrap().copy_into(target);
    }

    pub fn method(self, db: &Database, name: &str) -> Option<MethodId> {
        self.instance_of.method(db, name)
    }

    fn named_type(self, db: &Database, name: &str) -> Option<Symbol> {
        self.instance_of.named_type(db, name)
    }
}

/// A field for a class.
pub struct Field {
    index: usize,
    name: String,
    value_type: TypeRef,
    visibility: Visibility,
    module: ModuleId,
    location: Location,
    documentation: String,
}

impl Field {
    pub fn alloc(
        db: &mut Database,
        name: String,
        index: usize,
        value_type: TypeRef,
        visibility: Visibility,
        module: ModuleId,
        location: Location,
    ) -> FieldId {
        let id = db.fields.len();

        db.fields.push(Field {
            name,
            index,
            value_type,
            visibility,
            module,
            location,
            documentation: String::new(),
        });
        FieldId(id)
    }
}

/// An ID to a field.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct FieldId(pub usize);

impl FieldId {
    pub fn name(self, db: &Database) -> &String {
        &self.get(db).name
    }

    pub fn index(self, db: &Database) -> usize {
        self.get(db).index
    }

    pub fn value_type(self, db: &Database) -> TypeRef {
        self.get(db).value_type
    }

    pub fn set_value_type(self, db: &mut Database, value_type: TypeRef) {
        self.get_mut(db).value_type = value_type;
    }

    pub fn is_public(self, db: &Database) -> bool {
        self.get(db).visibility == Visibility::Public
    }

    pub fn is_visible_to(self, db: &Database, module: ModuleId) -> bool {
        let field = self.get(db);

        match field.visibility {
            Visibility::Public => true,
            Visibility::Private => {
                field.module.has_same_root_namespace(db, module)
            }
            // TypePrivate fields can only be accessed using the `@name` syntax,
            // which in turn is only available inside a class, thus not needing
            // any extra checks.
            Visibility::TypePrivate => false,
        }
    }

    pub fn location(self, db: &Database) -> Location {
        self.get(db).location
    }

    pub fn set_documentation(self, db: &mut Database, value: String) {
        self.get_mut(db).documentation = value;
    }

    pub fn documentation(self, db: &Database) -> &String {
        &self.get(db).documentation
    }

    fn get(self, db: &Database) -> &Field {
        &db.fields[self.0]
    }

    fn get_mut(self, db: &mut Database) -> &mut Field {
        &mut db.fields[self.0]
    }
}

/// Additional requirements for type parameters inside a trait implementation of
/// method.
///
/// This structure maps the original type parameters (`T` in this case) to type
/// parameters created for the bounds. These new type parameters have their
/// requirements set to the union of the original type parameter's requirements,
/// and the requirements specified in the bounds. In other words, if the
/// original parameter is defined as `T: A` and the bounds specify `T: B`, this
/// structure maps `T: A` to `T: A + B`.
#[derive(Clone, Debug)]
pub struct TypeBounds {
    mapping: HashMap<TypeParameterId, TypeParameterId>,
}

impl TypeBounds {
    pub fn new() -> Self {
        Self { mapping: HashMap::default() }
    }

    pub fn set(&mut self, parameter: TypeParameterId, bounds: TypeParameterId) {
        self.mapping.insert(parameter, bounds);
    }

    pub fn get(&self, parameter: TypeParameterId) -> Option<TypeParameterId> {
        self.mapping.get(&parameter).cloned()
    }

    pub fn iter(
        &self,
    ) -> impl Iterator<Item = (&TypeParameterId, &TypeParameterId)> {
        self.mapping.iter()
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut TypeParameterId> {
        self.mapping.values_mut()
    }

    pub fn is_empty(&self) -> bool {
        self.mapping.is_empty()
    }

    pub fn union(&self, with: &TypeBounds) -> TypeBounds {
        let mut union = self.clone();

        for (&key, &val) in &with.mapping {
            union.set(key, val);
        }

        union
    }

    pub fn make_immutable(&mut self, db: &mut Database) {
        for bound in self.mapping.values_mut() {
            if bound.is_mutable(db) {
                *bound = bound.as_immutable(db);
            }
        }
    }
}

/// An implementation of a trait, with (optionally) additional bounds for the
/// implementation.
#[derive(Clone)]
pub struct TraitImplementation {
    pub instance: TraitInstance,
    pub bounds: TypeBounds,
}

/// A single constructor defined in a enum class.
pub struct Constructor {
    id: u16,
    name: String,
    documentation: String,
    location: Location,
    arguments: Vec<TypeRef>,
}

impl Constructor {
    pub fn alloc(
        db: &mut Database,
        id: u16,
        name: String,
        members: Vec<TypeRef>,
        location: Location,
    ) -> ConstructorId {
        let global_id = db.constructors.len();

        db.constructors.push(Constructor {
            id,
            name,
            arguments: members,
            location,
            documentation: String::new(),
        });
        ConstructorId(global_id)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ConstructorId(pub usize);

impl ConstructorId {
    pub fn id(self, db: &Database) -> u16 {
        self.get(db).id
    }

    pub fn name(self, db: &Database) -> &String {
        &self.get(db).name
    }

    /// Returns the arguments of a constructor.
    ///
    /// The arguments are returned as a slice so one can inspect a constructor's
    /// arguments without always cloning the arguments.
    pub fn arguments(self, db: &Database) -> &[TypeRef] {
        &self.get(db).arguments
    }

    pub fn set_arguments(self, db: &mut Database, members: Vec<TypeRef>) {
        self.get_mut(db).arguments = members;
    }

    pub fn number_of_arguments(self, db: &Database) -> usize {
        self.get(db).arguments.len()
    }

    pub fn location(self, db: &Database) -> Location {
        self.get(db).location
    }

    pub fn set_documentation(self, db: &mut Database, value: String) {
        self.get_mut(db).documentation = value;
    }

    pub fn documentation(self, db: &Database) -> &String {
        &self.get(db).documentation
    }

    fn get(self, db: &Database) -> &Constructor {
        &db.constructors[self.0]
    }

    fn get_mut(self, db: &mut Database) -> &mut Constructor {
        &mut db.constructors[self.0]
    }
}

/// A type describing where something should be allocated.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum Storage {
    /// The value must be allocated on the heap.
    Heap,

    /// The value must be allocated inline/on the stack.
    Stack,
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum ClassKind {
    /// The type is an async type, aka a process.
    Async,

    /// The type is a type that uses atomic reference counting.
    Atomic,

    /// The type is a closure.
    Closure,

    /// The type is an enumeration.
    Enum,

    /// The type is a C structure.
    Extern,

    /// The type is a module.
    Module,

    /// The type is a regular user-defined type.
    Regular,

    /// The type is a N-arity tuple.
    Tuple,
}

impl ClassKind {
    pub fn is_async(self) -> bool {
        matches!(self, ClassKind::Async)
    }

    pub fn is_enum(self) -> bool {
        matches!(self, ClassKind::Enum)
    }

    pub fn is_tuple(self) -> bool {
        matches!(self, ClassKind::Tuple)
    }

    pub fn is_closure(self) -> bool {
        matches!(self, ClassKind::Closure)
    }

    pub fn is_module(self) -> bool {
        matches!(self, ClassKind::Module)
    }

    pub fn is_extern(self) -> bool {
        matches!(self, ClassKind::Extern)
    }

    pub fn allow_pattern_matching(self) -> bool {
        matches!(self, ClassKind::Regular | ClassKind::Extern)
    }

    fn is_atomic(self) -> bool {
        matches!(self, ClassKind::Async | ClassKind::Atomic)
    }
}

/// An Inko class as declared using the `class` keyword.
pub struct Class {
    kind: ClassKind,
    name: String,
    documentation: String,

    // A flag indicating the presence of a custom destructor.
    //
    // We store a flag for this so we can check for the presence of a destructor
    // without having to look up traits.
    destructor: bool,

    /// A type describing how instances of this type should be stored.
    storage: Storage,

    module: ModuleId,
    location: Location,
    visibility: Visibility,
    fields: IndexMap<String, FieldId>,
    type_parameters: IndexMap<String, TypeParameterId>,
    methods: HashMap<String, MethodId>,
    implemented_traits: HashMap<TraitId, TraitImplementation>,
    constructors: IndexMap<String, ConstructorId>,
    specializations: HashMap<Vec<Shape>, ClassId>,

    /// The ID of the class this class is a specialization of.
    specialization_source: Option<ClassId>,

    /// The shapes of the type parameters of this class, in the same order as
    /// the parameters.
    shapes: Vec<Shape>,
}

impl Class {
    pub fn alloc(
        db: &mut Database,
        name: String,
        kind: ClassKind,
        visibility: Visibility,
        module: ModuleId,
        location: Location,
    ) -> ClassId {
        let class = Class::new(name, kind, visibility, module, location);

        Class::add(db, class)
    }

    fn add(db: &mut Database, class: Class) -> ClassId {
        assert!(db.classes.len() < u32::MAX as usize);

        let id = db.classes.len() as u32;

        db.classes.push(class);
        ClassId(id)
    }

    fn new(
        name: String,
        kind: ClassKind,
        visibility: Visibility,
        module: ModuleId,
        location: Location,
    ) -> Self {
        let storage = if let ClassKind::Extern = kind {
            Storage::Stack
        } else {
            Storage::Heap
        };

        Self {
            name,
            documentation: String::new(),
            kind,
            visibility,
            storage,
            destructor: false,
            fields: IndexMap::new(),
            type_parameters: IndexMap::new(),
            methods: HashMap::new(),
            implemented_traits: HashMap::new(),
            constructors: IndexMap::new(),
            module,
            location,
            specializations: HashMap::new(),
            specialization_source: None,
            shapes: Vec::new(),
        }
    }

    fn regular(name: String) -> Self {
        Self::new(
            name,
            ClassKind::Regular,
            Visibility::Public,
            ModuleId(DEFAULT_BUILTIN_MODULE_ID),
            Location::default(),
        )
    }

    fn value_type(name: String) -> Self {
        let mut cls = Self::new(
            name,
            ClassKind::Regular,
            Visibility::Public,
            ModuleId(DEFAULT_BUILTIN_MODULE_ID),
            Location::default(),
        );

        cls.storage = Storage::Stack;
        cls
    }

    fn atomic(name: String) -> Self {
        Self::new(
            name,
            ClassKind::Atomic,
            Visibility::Public,
            ModuleId(DEFAULT_BUILTIN_MODULE_ID),
            Location::default(),
        )
    }

    fn tuple(name: String) -> Self {
        Self::new(
            name,
            ClassKind::Tuple,
            Visibility::Public,
            ModuleId(DEFAULT_BUILTIN_MODULE_ID),
            Location::default(),
        )
    }

    fn is_generic(&self) -> bool {
        !self.type_parameters.is_empty()
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct ClassId(pub u32);

impl ClassId {
    pub fn int() -> ClassId {
        ClassId(INT_ID)
    }

    pub fn float() -> ClassId {
        ClassId(FLOAT_ID)
    }

    pub fn string() -> ClassId {
        ClassId(STRING_ID)
    }

    pub fn boolean() -> ClassId {
        ClassId(BOOL_ID)
    }

    pub fn nil() -> ClassId {
        ClassId(NIL_ID)
    }

    pub fn array() -> ClassId {
        ClassId(ARRAY_ID)
    }

    pub fn byte_array() -> ClassId {
        ClassId(BYTE_ARRAY_ID)
    }

    pub fn tuple1() -> ClassId {
        ClassId(TUPLE1_ID)
    }

    pub fn tuple2() -> ClassId {
        ClassId(TUPLE2_ID)
    }

    pub fn tuple3() -> ClassId {
        ClassId(TUPLE3_ID)
    }

    pub fn tuple4() -> ClassId {
        ClassId(TUPLE4_ID)
    }

    pub fn tuple5() -> ClassId {
        ClassId(TUPLE5_ID)
    }

    pub fn tuple6() -> ClassId {
        ClassId(TUPLE6_ID)
    }

    pub fn tuple7() -> ClassId {
        ClassId(TUPLE7_ID)
    }

    pub fn tuple8() -> ClassId {
        ClassId(TUPLE8_ID)
    }

    pub fn checked_int_result() -> ClassId {
        ClassId(CHECKED_INT_RESULT_ID)
    }

    pub fn tuple(len: usize) -> Option<ClassId> {
        match len {
            1 => Some(ClassId::tuple1()),
            2 => Some(ClassId::tuple2()),
            3 => Some(ClassId::tuple3()),
            4 => Some(ClassId::tuple4()),
            5 => Some(ClassId::tuple5()),
            6 => Some(ClassId::tuple6()),
            7 => Some(ClassId::tuple7()),
            8 => Some(ClassId::tuple8()),
            _ => None,
        }
    }

    pub fn name(self, db: &Database) -> &String {
        &self.get(db).name
    }

    pub fn kind(self, db: &Database) -> ClassKind {
        self.get(db).kind
    }

    pub fn allow_trait_implementations(self, db: &Database) -> bool {
        !matches!(self.kind(db), ClassKind::Async | ClassKind::Extern)
    }

    pub fn type_parameters(self, db: &Database) -> Vec<TypeParameterId> {
        self.get(db).type_parameters.values().cloned().collect()
    }

    pub fn add_trait_implementation(
        self,
        db: &mut Database,
        implementation: TraitImplementation,
    ) {
        let trait_id = implementation.instance.instance_of();

        self.get_mut(db).implemented_traits.insert(trait_id, implementation);
        trait_id.get_mut(db).implemented_by.push(self);
    }

    pub fn trait_implementation(
        self,
        db: &Database,
        trait_type: TraitId,
    ) -> Option<&TraitImplementation> {
        self.get(db).implemented_traits.get(&trait_type)
    }

    pub fn implemented_traits(
        self,
        db: &Database,
    ) -> impl Iterator<Item = &TraitImplementation> {
        self.get(db).implemented_traits.values()
    }

    pub fn new_constructor(
        self,
        db: &mut Database,
        name: String,
        members: Vec<TypeRef>,
        location: Location,
    ) -> ConstructorId {
        let id = self.get(db).constructors.len() as u16;
        let constructor =
            Constructor::alloc(db, id, name.clone(), members, location);

        self.get_mut(db).constructors.insert(name, constructor);
        constructor
    }

    pub fn named_type(self, db: &Database, name: &str) -> Option<Symbol> {
        self.type_parameter(db, name).map(Symbol::TypeParameter)
    }

    pub fn type_parameter(
        self,
        db: &Database,
        name: &str,
    ) -> Option<TypeParameterId> {
        self.get(db).type_parameters.get(name).cloned()
    }

    pub fn field(self, db: &Database, name: &str) -> Option<FieldId> {
        self.get(db).fields.get(name).cloned()
    }

    pub fn field_by_index(
        self,
        db: &Database,
        index: usize,
    ) -> Option<FieldId> {
        self.get(db).fields.get_index(index).map(|(_, &v)| v)
    }

    pub fn field_names(self, db: &Database) -> Vec<String> {
        self.get(db).fields.keys().map(|k| k.to_string()).collect()
    }

    pub fn fields(self, db: &Database) -> Vec<FieldId> {
        self.get(db).fields.values().cloned().collect()
    }

    pub fn new_field(
        self,
        db: &mut Database,
        name: String,
        index: usize,
        value_type: TypeRef,
        visibility: Visibility,
        module: ModuleId,
        location: Location,
    ) -> FieldId {
        let id = Field::alloc(
            db,
            name.clone(),
            index,
            value_type,
            visibility,
            module,
            location,
        );

        self.get_mut(db).fields.insert(name, id);
        id
    }

    pub fn is_generic(self, db: &Database) -> bool {
        self.get(db).is_generic()
    }

    pub fn method(self, db: &Database, name: &str) -> Option<MethodId> {
        self.get(db).methods.get(name).cloned()
    }

    pub fn methods(self, db: &Database) -> Vec<MethodId> {
        self.get(db).methods.values().cloned().collect()
    }

    pub fn instance_methods(self, db: &Database) -> Vec<MethodId> {
        self.get(db)
            .methods
            .values()
            .filter(|v| v.is_instance(db))
            .cloned()
            .collect()
    }

    pub fn static_methods(self, db: &Database) -> Vec<MethodId> {
        self.get(db)
            .methods
            .values()
            .filter(|v| v.is_static(db))
            .cloned()
            .collect()
    }

    pub fn method_exists(self, db: &Database, name: &str) -> bool {
        self.get(db).methods.contains_key(name)
    }

    pub fn add_method(self, db: &mut Database, name: String, method: MethodId) {
        self.get_mut(db).methods.insert(name, method);
    }

    pub fn constructor(
        self,
        db: &Database,
        name: &str,
    ) -> Option<ConstructorId> {
        self.get(db).constructors.get(name).cloned()
    }

    pub fn constructors(self, db: &Database) -> Vec<ConstructorId> {
        self.get(db).constructors.values().cloned().collect()
    }

    pub fn number_of_constructors(self, db: &Database) -> usize {
        self.get(db).constructors.len()
    }

    pub fn number_of_fields(self, db: &Database) -> usize {
        self.get(db).fields.len()
    }

    pub fn number_of_methods(self, db: &Database) -> usize {
        self.get(db).methods.len()
    }

    pub fn enum_fields(self, db: &Database) -> Vec<FieldId> {
        let obj = self.get(db);

        if let ClassKind::Enum = obj.kind {
            // The first value is the tag, so we skip it.
            obj.fields[1..].values().cloned().collect()
        } else {
            Vec::new()
        }
    }

    pub fn is_public(self, db: &Database) -> bool {
        self.get(db).visibility == Visibility::Public
    }

    pub fn is_private(self, db: &Database) -> bool {
        !self.is_public(db)
    }

    pub fn is_atomic(self, db: &Database) -> bool {
        self.kind(db).is_atomic()
    }

    pub fn set_module(self, db: &mut Database, module: ModuleId) {
        self.get_mut(db).module = module;
    }

    pub fn module(self, db: &Database) -> ModuleId {
        self.get(db).module
    }

    pub fn set_shapes(self, db: &mut Database, shapes: Vec<Shape>) {
        self.get_mut(db).shapes = shapes;
    }

    pub fn specialization_source(self, db: &Database) -> Option<ClassId> {
        self.get(db).specialization_source
    }

    pub fn set_specialization_source(self, db: &mut Database, class: ClassId) {
        self.get_mut(db).specialization_source = Some(class);
    }

    pub fn specializations(
        self,
        db: &Database,
    ) -> &HashMap<Vec<Shape>, ClassId> {
        &self.get(db).specializations
    }

    pub fn shapes(self, db: &Database) -> &Vec<Shape> {
        &self.get(db).shapes
    }

    pub fn number_of_type_parameters(self, db: &Database) -> usize {
        self.get(db).type_parameters.len()
    }

    pub fn type_parameter_exists(self, db: &Database, name: &str) -> bool {
        self.get(db).type_parameters.contains_key(name)
    }

    pub fn new_type_parameter(
        self,
        db: &mut Database,
        name: String,
    ) -> TypeParameterId {
        let param = TypeParameter::alloc(db, name.clone());

        self.get_mut(db).type_parameters.insert(name, param);
        param
    }

    pub fn mark_as_having_destructor(self, db: &mut Database) {
        self.get_mut(db).destructor = true;
    }

    pub fn has_destructor(self, db: &Database) -> bool {
        self.get(db).destructor
    }

    pub fn is_builtin(self) -> bool {
        self.0 <= NIL_ID
    }

    pub fn is_value_type(self, db: &Database) -> bool {
        let typ = self.get(db);

        match typ.kind {
            // These types are allocated on the heap but treated as value types.
            ClassKind::Async | ClassKind::Atomic => true,
            _ => matches!(typ.storage, Storage::Stack),
        }
    }

    pub fn is_heap_allocated(self, db: &Database) -> bool {
        matches!(self.get(db).storage, Storage::Heap)
    }

    pub fn is_stack_allocated(self, db: &Database) -> bool {
        matches!(self.get(db).storage, Storage::Stack)
    }

    pub fn has_object_header(self, db: &Database) -> bool {
        // Currently heap objects always have an object header and stack objects
        // never have one, but this may change at some point. For example, if an
        // object is somehow promoted from the heap to the stack it might retain
        // its header. Using a separate method for this helps future proof
        // things.
        !self.is_stack_allocated(db)
    }

    pub fn is_closure(self, db: &Database) -> bool {
        self.kind(db).is_closure()
    }

    pub fn is_numeric(self) -> bool {
        matches!(self.0, INT_ID | FLOAT_ID)
    }

    pub fn allow_cast_to_trait(self, db: &Database) -> bool {
        let typ = self.get(db);

        match typ.kind {
            // Only heap allocated versions of these types have a header and
            // thus can be casted to a trait.
            ClassKind::Enum | ClassKind::Regular | ClassKind::Tuple => {
                matches!(typ.storage, Storage::Heap)
            }
            // Other types such as closures, processes and extern classes can't
            // ever be casted to a trait.
            _ => false,
        }
    }

    pub fn allow_cast_to_foreign(self, db: &Database) -> bool {
        matches!(self.get(db).storage, Storage::Heap)
            || matches!(self.0, INT_ID | FLOAT_ID | BOOL_ID)
    }

    pub fn documentation(self, db: &Database) -> &String {
        &self.get(db).documentation
    }

    pub fn set_documentation(self, db: &mut Database, value: String) {
        self.get_mut(db).documentation = value;
    }

    pub fn location(self, db: &Database) -> Location {
        self.get(db).location
    }

    pub fn set_location(self, db: &mut Database, value: Location) {
        self.get_mut(db).location = value;
    }

    pub fn set_stack_allocated(self, db: &mut Database) {
        self.get_mut(db).storage = Storage::Stack;
    }

    pub fn clone_for_specialization(self, db: &mut Database) -> ClassId {
        let src = self.get(db);
        let mut new = Class::new(
            src.name.clone(),
            src.kind,
            src.visibility,
            src.module,
            src.location,
        );

        new.storage = src.storage;
        Class::add(db, new)
    }

    pub fn allow_mutating(self, db: &Database) -> bool {
        let obj = self.get(db);

        match obj.kind {
            ClassKind::Extern => true,
            ClassKind::Atomic => false,
            _ => matches!(obj.storage, Storage::Heap),
        }
    }

    fn get(self, db: &Database) -> &Class {
        &db.classes[self.0 as usize]
    }

    fn get_mut(self, db: &mut Database) -> &mut Class {
        &mut db.classes[self.0 as usize]
    }
}

/// An instance of a class, along with its type arguments in case the class is
/// generic.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ClassInstance {
    /// The ID of the class we're an instance of.
    instance_of: ClassId,

    /// The index to the TypeArguments structure that belongs to this class
    /// instance.
    ///
    /// If the class isn't generic, this index shouldn't be used to obtain the
    /// type arguments, as it won't be used.
    ///
    /// After type specialization takes place, this value shouldn't be used any
    /// more as specialized types won't have their type arguments set.
    type_arguments: u32,
}

impl ClassInstance {
    pub fn new(instance_of: ClassId) -> Self {
        Self { instance_of, type_arguments: 0 }
    }

    pub fn rigid(
        db: &mut Database,
        instance_of: ClassId,
        bounds: &TypeBounds,
    ) -> Self {
        if instance_of.is_generic(db) {
            let mut arguments = TypeArguments::new();

            for param in instance_of.type_parameters(db) {
                arguments.assign(
                    param,
                    bounds.get(param).unwrap_or(param).as_rigid(),
                );
            }

            Self::generic(db, instance_of, arguments)
        } else {
            Self::new(instance_of)
        }
    }

    pub fn generic(
        db: &mut Database,
        instance_of: ClassId,
        arguments: TypeArguments,
    ) -> Self {
        assert!(db.type_arguments.len() < u32::MAX as usize);

        let args_id = db.type_arguments.len() as u32;

        db.type_arguments.push(arguments);
        ClassInstance { instance_of, type_arguments: args_id }
    }

    pub fn with_types(
        db: &mut Database,
        class: ClassId,
        arguments: Vec<TypeRef>,
    ) -> Self {
        let mut args = TypeArguments::new();

        for (index, param) in class.type_parameters(db).into_iter().enumerate()
        {
            let val = arguments
                .get(index)
                .cloned()
                .unwrap_or_else(|| TypeRef::placeholder(db, Some(param)));

            args.assign(param, val);
        }

        Self::generic(db, class, args)
    }

    pub fn empty(db: &mut Database, class: ClassId) -> Self {
        if !class.is_generic(db) {
            return Self::new(class);
        }

        let mut args = TypeArguments::new();

        for param in class.type_parameters(db) {
            args.assign(param, TypeRef::placeholder(db, Some(param)));
        }

        Self::generic(db, class, args)
    }

    pub fn instance_of(self) -> ClassId {
        self.instance_of
    }

    pub fn type_arguments(self, db: &Database) -> Option<&TypeArguments> {
        db.type_arguments.get(self.type_arguments as usize)
    }

    pub fn method(self, db: &Database, name: &str) -> Option<MethodId> {
        self.instance_of.method(db, name)
    }

    pub fn ordered_type_arguments(self, db: &Database) -> Vec<TypeRef> {
        let params = self.instance_of.type_parameters(db);
        let args = self.type_arguments(db).unwrap();

        params
            .into_iter()
            .map(|p| args.get(p).unwrap_or(TypeRef::Unknown))
            .collect()
    }

    fn shape(
        self,
        db: &Database,
        interned: &mut InternedTypeArguments,
        default: Shape,
    ) -> Shape {
        match self.instance_of.0 {
            INT_ID => Shape::int(),
            FLOAT_ID => Shape::float(),
            BOOL_ID => Shape::Boolean,
            NIL_ID => Shape::Nil,
            STRING_ID => Shape::String,
            _ if self.instance_of.kind(db).is_atomic() => Shape::Atomic,
            _ if self.instance_of.is_stack_allocated(db) => {
                let targs = if self.instance_of.is_generic(db) {
                    // We need to make sure that for different references to the
                    // same type (e.g. `SomeType[Int]`), the type arguments ID
                    // is the same so we can reliable compare and hash the
                    // returned Shape.
                    interned.intern(db, self)
                } else {
                    0
                };

                Shape::Stack(ClassInstance {
                    instance_of: self.instance_of,
                    type_arguments: targs,
                })
            }
            _ => default,
        }
    }

    fn named_type(self, db: &Database, name: &str) -> Option<Symbol> {
        self.instance_of.named_type(db, name)
    }
}

/// A collection of arguments.
#[derive(Clone)]
struct Arguments {
    mapping: IndexMap<String, Argument>,
}

impl Arguments {
    fn new() -> Self {
        Self { mapping: IndexMap::new() }
    }

    fn new_argument(
        &mut self,
        name: String,
        value_type: TypeRef,
        variable: VariableId,
    ) {
        let index = self.mapping.len();
        let arg = Argument { index, name: name.clone(), value_type, variable };

        // Since indexes of arguments are incremented with insertions, we need
        // to make sure we're not updating an existing argument by mistake.
        debug_assert!(self.mapping.get(&name).is_none());
        self.mapping.insert(name, arg);
    }

    fn get(&self, name: &str) -> Option<&Argument> {
        self.mapping.get(name)
    }

    fn iter(&self) -> impl Iterator<Item = &Argument> {
        self.mapping.values()
    }

    fn len(&self) -> usize {
        self.mapping.len()
    }
}

/// An argument defined in a method or closure.
#[derive(Clone)]
pub struct Argument {
    pub index: usize,
    pub name: String,
    pub value_type: TypeRef,
    pub variable: VariableId,
}

/// A block of code, such as a closure or method.
pub trait Block {
    fn new_argument(
        &self,
        db: &mut Database,
        name: String,
        variable_type: TypeRef,
        argument_type: TypeRef,
        location: Location,
    ) -> VariableId;
    fn return_type(&self, db: &Database) -> TypeRef;
    fn set_return_type(&self, db: &mut Database, typ: TypeRef);
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Visibility {
    /// A public symbol can be used by any other module.
    Public,

    /// A symbol only available to the module in which it's defined.
    Private,

    /// A symbol only available inside the type that defined it.
    TypePrivate,
}

impl Visibility {
    pub fn public(public: bool) -> Visibility {
        if public {
            Self::Public
        } else {
            Self::Private
        }
    }

    pub fn is_private(self) -> bool {
        self != Self::Public
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Intrinsic {
    FloatAdd,
    FloatCeil,
    FloatDiv,
    FloatEq,
    FloatFloor,
    FloatFromBits,
    FloatGe,
    FloatGt,
    FloatIsInf,
    FloatIsNan,
    FloatLe,
    FloatLt,
    FloatMod,
    FloatMul,
    FloatSub,
    FloatToBits,
    IntBitAnd,
    IntBitNot,
    IntBitOr,
    IntBitXor,
    IntDiv,
    IntEq,
    IntGe,
    IntGt,
    IntLe,
    IntLt,
    IntRem,
    IntRotateLeft,
    IntRotateRight,
    IntShl,
    IntShr,
    IntUnsignedShr,
    IntWrappingAdd,
    IntWrappingMul,
    IntWrappingSub,
    Moved,
    Panic,
    StringConcat,
    State,
    Process,
    FloatRound,
    FloatPowi,
    IntCheckedAdd,
    IntCheckedMul,
    IntCheckedSub,
    IntSwapBytes,
    IntAbsolute,
    IntCompareSwap,
    SpinLoopHint,
    BoolEq,
}

impl Intrinsic {
    pub fn mapping() -> HashMap<String, Self> {
        vec![
            Intrinsic::FloatAdd,
            Intrinsic::FloatCeil,
            Intrinsic::FloatDiv,
            Intrinsic::FloatEq,
            Intrinsic::FloatFloor,
            Intrinsic::FloatFromBits,
            Intrinsic::FloatGe,
            Intrinsic::FloatGt,
            Intrinsic::FloatIsInf,
            Intrinsic::FloatIsNan,
            Intrinsic::FloatLe,
            Intrinsic::FloatLt,
            Intrinsic::FloatMod,
            Intrinsic::FloatMul,
            Intrinsic::FloatSub,
            Intrinsic::FloatToBits,
            Intrinsic::IntBitAnd,
            Intrinsic::IntBitNot,
            Intrinsic::IntBitOr,
            Intrinsic::IntBitXor,
            Intrinsic::IntDiv,
            Intrinsic::IntEq,
            Intrinsic::IntGe,
            Intrinsic::IntGt,
            Intrinsic::IntLe,
            Intrinsic::IntLt,
            Intrinsic::IntRem,
            Intrinsic::IntRotateLeft,
            Intrinsic::IntRotateRight,
            Intrinsic::IntShl,
            Intrinsic::IntShr,
            Intrinsic::IntUnsignedShr,
            Intrinsic::IntWrappingAdd,
            Intrinsic::IntWrappingMul,
            Intrinsic::IntWrappingSub,
            Intrinsic::IntCheckedAdd,
            Intrinsic::IntCheckedMul,
            Intrinsic::IntCheckedSub,
            Intrinsic::Moved,
            Intrinsic::Panic,
            Intrinsic::StringConcat,
            Intrinsic::State,
            Intrinsic::Process,
            Intrinsic::FloatRound,
            Intrinsic::FloatPowi,
            Intrinsic::IntSwapBytes,
            Intrinsic::IntAbsolute,
            Intrinsic::IntCompareSwap,
            Intrinsic::SpinLoopHint,
            Intrinsic::BoolEq,
        ]
        .into_iter()
        .fold(HashMap::new(), |mut map, func| {
            map.insert(func.name().to_string(), func);
            map
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Intrinsic::FloatAdd => "float_add",
            Intrinsic::FloatCeil => "float_ceil",
            Intrinsic::FloatDiv => "float_div",
            Intrinsic::FloatEq => "float_eq",
            Intrinsic::FloatFloor => "float_floor",
            Intrinsic::FloatFromBits => "float_from_bits",
            Intrinsic::FloatGe => "float_ge",
            Intrinsic::FloatGt => "float_gt",
            Intrinsic::FloatIsInf => "float_is_inf",
            Intrinsic::FloatIsNan => "float_is_nan",
            Intrinsic::FloatLe => "float_le",
            Intrinsic::FloatLt => "float_lt",
            Intrinsic::FloatMod => "float_mod",
            Intrinsic::FloatMul => "float_mul",
            Intrinsic::FloatSub => "float_sub",
            Intrinsic::FloatToBits => "float_to_bits",
            Intrinsic::IntBitAnd => "int_bit_and",
            Intrinsic::IntBitNot => "int_bit_not",
            Intrinsic::IntBitOr => "int_bit_or",
            Intrinsic::IntBitXor => "int_bit_xor",
            Intrinsic::IntDiv => "int_div",
            Intrinsic::IntEq => "int_eq",
            Intrinsic::IntGe => "int_ge",
            Intrinsic::IntGt => "int_gt",
            Intrinsic::IntLe => "int_le",
            Intrinsic::IntLt => "int_lt",
            Intrinsic::IntRem => "int_rem",
            Intrinsic::IntRotateLeft => "int_rotate_left",
            Intrinsic::IntRotateRight => "int_rotate_right",
            Intrinsic::IntShl => "int_shl",
            Intrinsic::IntShr => "int_shr",
            Intrinsic::IntUnsignedShr => "int_unsigned_shr",
            Intrinsic::IntWrappingAdd => "int_wrapping_add",
            Intrinsic::IntWrappingMul => "int_wrapping_mul",
            Intrinsic::IntWrappingSub => "int_wrapping_sub",
            Intrinsic::IntCheckedAdd => "int_checked_add",
            Intrinsic::IntCheckedMul => "int_checked_mul",
            Intrinsic::IntCheckedSub => "int_checked_sub",
            Intrinsic::Moved => "moved",
            Intrinsic::Panic => "panic",
            Intrinsic::StringConcat => "string_concat",
            Intrinsic::State => "state",
            Intrinsic::Process => "process",
            Intrinsic::FloatRound => "float_round",
            Intrinsic::FloatPowi => "float_powi",
            Intrinsic::IntSwapBytes => "int_swap_bytes",
            Intrinsic::IntAbsolute => "int_absolute",
            Intrinsic::IntCompareSwap => "int_compare_swap",
            Intrinsic::SpinLoopHint => "spin_loop_hint",
            Intrinsic::BoolEq => "bool_eq",
        }
    }

    pub fn return_type(self) -> TypeRef {
        let checked_result = TypeRef::Owned(TypeId::ClassInstance(
            ClassInstance::new(ClassId::checked_int_result()),
        ));

        match self {
            Intrinsic::FloatAdd => TypeRef::float(),
            Intrinsic::FloatCeil => TypeRef::float(),
            Intrinsic::FloatDiv => TypeRef::float(),
            Intrinsic::FloatEq => TypeRef::boolean(),
            Intrinsic::FloatFloor => TypeRef::float(),
            Intrinsic::FloatFromBits => TypeRef::float(),
            Intrinsic::FloatGe => TypeRef::boolean(),
            Intrinsic::FloatGt => TypeRef::boolean(),
            Intrinsic::FloatIsInf => TypeRef::boolean(),
            Intrinsic::FloatIsNan => TypeRef::boolean(),
            Intrinsic::FloatLe => TypeRef::boolean(),
            Intrinsic::FloatLt => TypeRef::boolean(),
            Intrinsic::FloatMod => TypeRef::float(),
            Intrinsic::FloatMul => TypeRef::float(),
            Intrinsic::FloatSub => TypeRef::float(),
            Intrinsic::FloatToBits => TypeRef::int(),
            Intrinsic::IntBitAnd => TypeRef::int(),
            Intrinsic::IntBitNot => TypeRef::int(),
            Intrinsic::IntBitOr => TypeRef::int(),
            Intrinsic::IntBitXor => TypeRef::int(),
            Intrinsic::IntDiv => TypeRef::int(),
            Intrinsic::IntEq => TypeRef::boolean(),
            Intrinsic::IntGe => TypeRef::boolean(),
            Intrinsic::IntGt => TypeRef::boolean(),
            Intrinsic::IntLe => TypeRef::boolean(),
            Intrinsic::IntLt => TypeRef::boolean(),
            Intrinsic::IntRem => TypeRef::int(),
            Intrinsic::IntRotateLeft => TypeRef::int(),
            Intrinsic::IntRotateRight => TypeRef::int(),
            Intrinsic::IntShl => TypeRef::int(),
            Intrinsic::IntShr => TypeRef::int(),
            Intrinsic::IntUnsignedShr => TypeRef::int(),
            Intrinsic::IntWrappingAdd => TypeRef::int(),
            Intrinsic::IntWrappingMul => TypeRef::int(),
            Intrinsic::IntWrappingSub => TypeRef::int(),
            Intrinsic::IntCheckedAdd => checked_result,
            Intrinsic::IntCheckedMul => checked_result,
            Intrinsic::IntCheckedSub => checked_result,
            Intrinsic::Moved => TypeRef::nil(),
            Intrinsic::Panic => TypeRef::Never,
            Intrinsic::StringConcat => TypeRef::string(),
            Intrinsic::State => TypeRef::pointer(TypeId::Foreign(
                ForeignType::Int(8, Sign::Unsigned),
            )),
            Intrinsic::Process => TypeRef::pointer(TypeId::Foreign(
                ForeignType::Int(8, Sign::Unsigned),
            )),
            Intrinsic::FloatRound => TypeRef::float(),
            Intrinsic::FloatPowi => TypeRef::float(),
            Intrinsic::IntSwapBytes => TypeRef::int(),
            Intrinsic::IntAbsolute => TypeRef::int(),
            Intrinsic::IntCompareSwap => TypeRef::boolean(),
            Intrinsic::SpinLoopHint => TypeRef::nil(),
            Intrinsic::BoolEq => TypeRef::boolean(),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum MethodKind {
    /// An immutable asynchronous method.
    Async,

    /// A mutable asynchronous method.
    AsyncMutable,

    /// A static method.
    Static,

    /// A static method generated for an enum constructor.
    Constructor,

    /// A regular immutable instance method.
    Instance,

    /// An instance method that takes ownership of its receiver.
    Moving,

    /// A mutable instance method.
    Mutable,

    /// The method is a destructor.
    Destructor,

    /// The method is an external/FFI function.
    Extern,
}

impl MethodKind {
    fn is_static(self) -> bool {
        matches!(self, MethodKind::Static | MethodKind::Constructor)
    }

    pub fn is_constructor(self) -> bool {
        matches!(self, MethodKind::Constructor)
    }
}

#[derive(Copy, Clone)]
pub enum MethodSource {
    /// The method is directly defined for a type.
    Direct,

    /// The method is a default method implemented through a trait.
    Implemented(TraitInstance, MethodId),

    /// The method is a default method that was inherited by not overwriting it.
    Inherited(TraitInstance, MethodId),
}

pub enum MethodLookup {
    /// The method lookup is valid.
    Ok(MethodId),

    /// The method exists, but it's private and unavailable to the caller.
    Private,

    /// The method exists, but it's an instance method and the receiver is not
    /// an instance.
    InstanceOnStatic,

    /// The method exists, but it's a static method and the receiver is an
    /// instance.
    StaticOnInstance,

    /// The method doesn't exist.
    None,
}

/// The call convention of a method.
#[derive(Copy, Clone)]
pub enum CallConvention {
    Inko,
    C,
}

impl CallConvention {
    pub fn new(c: bool) -> CallConvention {
        if c {
            CallConvention::C
        } else {
            CallConvention::Inko
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum Inline {
    /// The method must never be inlined.
    Never,

    /// The need for inlining should be determined based on some set of
    /// heuristics.
    Infer,

    /// The method must be inlined into every static call site.
    Always,
}

/// A static or instance method.
#[derive(Clone)]
pub struct Method {
    module: ModuleId,
    location: Location,
    name: String,
    documentation: String,
    kind: MethodKind,
    call_convention: CallConvention,
    visibility: Visibility,
    inline: Inline,
    type_parameters: IndexMap<String, TypeParameterId>,
    arguments: Arguments,
    bounds: TypeBounds,
    return_type: TypeRef,
    source: MethodSource,
    main: bool,
    variadic: bool,

    /// The type of the receiver of the method.
    receiver: TypeRef,

    /// The fields this method has access to, along with their types.
    field_types: HashMap<String, (FieldId, TypeRef)>,

    /// The specializations of this method, if the method itself is generic.
    ///
    /// Each key is the combination of the receiver and method shapes, in the
    /// same order as their type parameters.
    specializations: HashMap<Vec<Shape>, MethodId>,

    /// The shapes of this method's type parameters, if any.
    ///
    /// For static methods this list starts with the shapes of the surrounding
    /// class' type parameters, if any. For instance methods, we only include
    /// the shapes of the method's type parameters.
    shapes: Vec<Shape>,
}

impl Method {
    pub fn alloc(
        db: &mut Database,
        module: ModuleId,
        location: Location,
        name: String,
        visibility: Visibility,
        kind: MethodKind,
    ) -> MethodId {
        assert!(db.methods.len() < METHODS_LIMIT);

        let mut call_convention = CallConvention::Inko;
        let mut inline = Inline::Infer;

        if let MethodKind::Extern = kind {
            call_convention = CallConvention::C;

            // External functions are never inlined because they're either
            // defined externally (such that there's nothing to inline) _or_
            // they're meant to be called from C code, in which case the
            // function _must_ in fact exist in generated code.
            inline = Inline::Never;
        }

        let id = db.methods.len();
        let method = Method {
            module,
            location,
            name,
            kind,
            call_convention,
            visibility,
            documentation: String::new(),
            type_parameters: IndexMap::new(),
            bounds: TypeBounds::new(),
            arguments: Arguments::new(),
            return_type: TypeRef::Unknown,
            source: MethodSource::Direct,
            receiver: TypeRef::Unknown,
            field_types: HashMap::new(),
            main: false,
            variadic: false,
            specializations: HashMap::new(),
            shapes: Vec::new(),
            inline,
        };

        db.methods.push(method);
        MethodId(id as _)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct MethodId(pub u32);

impl MethodId {
    pub fn is_generated(self, db: &Database) -> bool {
        self.get(db).name.starts_with('$')
    }

    pub fn named_type(self, db: &Database, name: &str) -> Option<Symbol> {
        self.get(db)
            .type_parameters
            .get(name)
            .map(|&id| Symbol::TypeParameter(id))
    }

    pub fn type_parameters(self, db: &Database) -> Vec<TypeParameterId> {
        self.get(db).type_parameters.values().cloned().collect()
    }

    pub fn new_type_parameter(
        self,
        db: &mut Database,
        name: String,
    ) -> TypeParameterId {
        let param = TypeParameter::alloc(db, name.clone());

        self.get_mut(db).type_parameters.insert(name, param);
        param
    }

    pub fn set_module(self, db: &mut Database, module: ModuleId) {
        self.get_mut(db).module = module;
    }

    pub fn set_receiver(self, db: &mut Database, receiver: TypeRef) {
        self.get_mut(db).receiver = receiver;
    }

    pub fn receiver_for_class_instance(
        self,
        db: &Database,
        instance: ClassInstance,
    ) -> TypeRef {
        let rec_id = TypeId::ClassInstance(instance);

        match self.kind(db) {
            // Async methods always access `self` through a reference even
            // though processes are value types. This way we prevent immutable
            // async methods from being able to mutate the process' internal
            // state.
            MethodKind::Async => TypeRef::Ref(rec_id),
            MethodKind::AsyncMutable => TypeRef::Mut(rec_id),

            // For regular value types (e.g. Int), `self` is always an owned
            // value.
            _ if self.receiver(db).is_value_type(db)
                && !instance.instance_of().kind(db).is_async() =>
            {
                TypeRef::Owned(rec_id)
            }
            MethodKind::Instance => TypeRef::Ref(rec_id),
            MethodKind::Mutable | MethodKind::Destructor => {
                TypeRef::Mut(rec_id)
            }
            MethodKind::Static | MethodKind::Constructor => {
                TypeRef::Owned(TypeId::Class(instance.instance_of()))
            }
            MethodKind::Moving => TypeRef::Owned(rec_id),
            MethodKind::Extern => TypeRef::Unknown,
        }
    }

    pub fn receiver(self, db: &Database) -> TypeRef {
        self.get(db).receiver
    }

    pub fn receiver_id(self, db: &Database) -> TypeId {
        self.get(db).receiver.type_id(db).unwrap()
    }

    pub fn source(self, db: &Database) -> MethodSource {
        self.get(db).source
    }

    pub fn set_source(self, db: &mut Database, source: MethodSource) {
        self.get_mut(db).source = source;
    }

    pub fn name(self, db: &Database) -> &String {
        &self.get(db).name
    }

    pub fn is_private(self, db: &Database) -> bool {
        !self.is_public(db)
    }

    pub fn is_public(self, db: &Database) -> bool {
        self.get(db).visibility == Visibility::Public
    }

    pub fn location(self, db: &Database) -> Location {
        self.get(db).location
    }

    pub fn set_documentation(self, db: &mut Database, value: String) {
        self.get_mut(db).documentation = value;
    }

    pub fn documentation(self, db: &Database) -> &String {
        let method = self.get(db);

        if method.documentation.is_empty() {
            // For methods implemented through a trait, the documentation is
            // inherited if not overwritten explicitly.
            if let Some(id) = self.original_method(db) {
                id.documentation(db)
            } else {
                &method.documentation
            }
        } else {
            &method.documentation
        }
    }

    pub fn is_mutable(self, db: &Database) -> bool {
        matches!(self.kind(db), MethodKind::Mutable | MethodKind::AsyncMutable)
    }

    pub fn is_immutable(self, db: &Database) -> bool {
        matches!(
            self.kind(db),
            MethodKind::Async | MethodKind::Static | MethodKind::Instance
        )
    }

    pub fn is_async(self, db: &Database) -> bool {
        matches!(self.kind(db), MethodKind::Async | MethodKind::AsyncMutable)
    }

    pub fn is_static(self, db: &Database) -> bool {
        matches!(self.kind(db), MethodKind::Static | MethodKind::Constructor)
    }

    pub fn is_extern(self, db: &Database) -> bool {
        matches!(self.kind(db), MethodKind::Extern)
    }

    pub fn is_moving(self, db: &Database) -> bool {
        matches!(self.kind(db), MethodKind::Moving)
    }

    pub fn is_instance(self, db: &Database) -> bool {
        matches!(
            self.kind(db),
            MethodKind::Async
                | MethodKind::AsyncMutable
                | MethodKind::Instance
                | MethodKind::Moving
                | MethodKind::Mutable
                | MethodKind::Destructor
        )
    }

    pub fn set_variadic(self, db: &mut Database) {
        self.get_mut(db).variadic = true;
    }

    pub fn is_variadic(self, db: &Database) -> bool {
        self.get(db).variadic
    }

    pub fn positional_argument_input_type(
        self,
        db: &Database,
        index: usize,
    ) -> Option<TypeRef> {
        self.get(db)
            .arguments
            .mapping
            .get_index(index)
            .map(|(_, a)| a.value_type)
    }

    pub fn arguments(self, db: &Database) -> Vec<Argument> {
        self.get(db).arguments.mapping.values().cloned().collect()
    }

    pub fn argument_types(
        self,
        db: &Database,
    ) -> impl Iterator<Item = &TypeRef> {
        self.get(db).arguments.mapping.values().map(|a| &a.value_type)
    }

    pub fn update_argument_types(
        self,
        db: &mut Database,
        index: usize,
        variable_type: TypeRef,
        argument_type: TypeRef,
    ) {
        let arg = &mut self.get_mut(db).arguments.mapping[index];

        arg.value_type = argument_type;
        arg.variable.set_value_type(db, variable_type);
    }

    pub fn named_argument(
        self,
        db: &Database,
        name: &str,
    ) -> Option<(usize, TypeRef)> {
        self.get(db).arguments.get(name).map(|a| (a.index, a.value_type))
    }

    pub fn number_of_arguments(self, db: &Database) -> usize {
        self.get(db).arguments.len()
    }

    pub fn copy_method(self, db: &mut Database, module: ModuleId) -> MethodId {
        assert!(db.methods.len() < u32::MAX as usize);

        let mut copy = self.get(db).clone();
        let id = db.methods.len();

        copy.module = module;
        db.methods.push(copy);
        MethodId(id as _)
    }

    pub fn mark_as_destructor(self, db: &mut Database) {
        let method = self.get_mut(db);

        method.kind = MethodKind::Destructor;

        // Since destructors are always called through dropper methods, we want
        // to avoid the extra function call, so we inline these into their
        // droppers.
        method.inline = Inline::Always;
    }

    pub fn kind(self, db: &Database) -> MethodKind {
        self.get(db).kind
    }

    pub fn module(self, db: &Database) -> ModuleId {
        self.get(db).module
    }

    pub fn ignore_return_value(self, db: &Database) -> bool {
        self.get(db).return_type == TypeRef::nil()
    }

    pub fn set_field_type(
        self,
        db: &mut Database,
        name: String,
        id: FieldId,
        value_type: TypeRef,
    ) {
        self.get_mut(db).field_types.insert(name, (id, value_type));
    }

    pub fn field_id_and_type(
        self,
        db: &Database,
        name: &str,
    ) -> Option<(FieldId, TypeRef)> {
        self.get(db).field_types.get(name).cloned()
    }

    pub fn fields(self, db: &Database) -> Vec<(FieldId, TypeRef)> {
        self.get(db).field_types.values().cloned().collect()
    }

    pub fn add_argument(self, db: &mut Database, argument: Argument) {
        self.get_mut(db).arguments.new_argument(
            argument.name.clone(),
            argument.value_type,
            argument.variable,
        );
    }

    pub fn set_main(self, db: &mut Database) {
        self.get_mut(db).main = true;
    }

    pub fn is_main(self, db: &Database) -> bool {
        self.get(db).main
    }

    pub fn bounds(self, db: &Database) -> &TypeBounds {
        &self.get(db).bounds
    }

    pub fn set_bounds(self, db: &mut Database, bounds: TypeBounds) {
        self.get_mut(db).bounds = bounds;
    }

    pub fn has_return_type(self, db: &Database) -> bool {
        let method = self.get(db);

        if matches!(method.call_convention, CallConvention::C) {
            method.return_type != TypeRef::nil()
        } else {
            true
        }
    }

    pub fn returns_value(self, db: &Database) -> bool {
        self.has_return_type(db) && !self.return_type(db).is_never(db)
    }

    pub fn add_specialization(
        self,
        db: &mut Database,
        shapes: Vec<Shape>,
        method: MethodId,
    ) {
        self.get_mut(db).specializations.insert(shapes, method);
    }

    pub fn set_shapes(self, db: &mut Database, shapes: Vec<Shape>) {
        self.get_mut(db).shapes = shapes;
    }

    pub fn shapes(self, db: &Database) -> &Vec<Shape> {
        &self.get(db).shapes
    }

    pub fn specialization(
        self,
        db: &Database,
        shapes: &[Shape],
    ) -> Option<MethodId> {
        self.get(db).specializations.get(shapes).cloned()
    }

    pub fn specializations(self, db: &Database) -> Vec<MethodId> {
        self.get(db).specializations.values().cloned().collect()
    }

    pub fn clone_for_specialization(self, db: &mut Database) -> MethodId {
        let (module, location, name, vis, kind, source, inline) = {
            let old = self.get(db);

            (
                old.module,
                old.location,
                old.name.clone(),
                old.visibility,
                old.kind,
                old.source,
                old.inline,
            )
        };

        let new = Method::alloc(db, module, location, name, vis, kind);

        new.set_source(db, source);
        new.set_inline(db, inline);
        new
    }

    pub fn is_generic(self, db: &Database) -> bool {
        !self.get(db).type_parameters.is_empty()
    }

    pub fn original_method(self, db: &Database) -> Option<MethodId> {
        match self.get(db).source {
            MethodSource::Direct => None,
            MethodSource::Implemented(_, v) | MethodSource::Inherited(_, v) => {
                Some(v)
            }
        }
    }

    pub fn implemented_trait_instance(
        self,
        db: &Database,
    ) -> Option<TraitInstance> {
        match self.get(db).source {
            MethodSource::Direct => None,
            MethodSource::Implemented(v, _) | MethodSource::Inherited(v, _) => {
                Some(v)
            }
        }
    }

    /// Returns the module in which the method is defined.
    ///
    /// For default trait methods that aren't overwritten, this returns the
    /// module in which the trait is defined, _not_ the module in which it was
    /// implemented.
    pub fn source_module(self, db: &Database) -> ModuleId {
        let m = self.get(db);

        match m.source {
            MethodSource::Direct | MethodSource::Implemented(_, _) => m.module,
            MethodSource::Inherited(ins, _) => ins.instance_of().module(db),
        }
    }

    /// Returns the file path in which the method is defined.
    ///
    /// For default trait methods that aren't overwritten, this returns the path
    /// of the module the trait is defined in.
    pub fn source_file(self, db: &Database) -> PathBuf {
        self.source_module(db).file(db)
    }

    pub fn uses_c_calling_convention(self, db: &Database) -> bool {
        matches!(self.get(db).call_convention, CallConvention::C)
    }

    pub fn use_c_calling_convention(self, db: &mut Database) {
        self.get_mut(db).call_convention = CallConvention::C;
    }

    pub fn call_convention(self, db: &Database) -> CallConvention {
        self.get(db).call_convention
    }

    pub fn always_inline(self, db: &mut Database) {
        self.get_mut(db).inline = Inline::Always;
    }

    pub fn set_inline(self, db: &mut Database, inline: Inline) {
        self.get_mut(db).inline = inline;
    }

    pub fn inline(self, db: &Database) -> Inline {
        self.get(db).inline
    }

    fn get(self, db: &Database) -> &Method {
        &db.methods[self.0 as usize]
    }

    fn get_mut(self, db: &mut Database) -> &mut Method {
        &mut db.methods[self.0 as usize]
    }
}

impl Block for MethodId {
    fn new_argument(
        &self,
        db: &mut Database,
        name: String,
        variable_type: TypeRef,
        argument_type: TypeRef,
        location: Location,
    ) -> VariableId {
        let var =
            Variable::alloc(db, name.clone(), variable_type, false, location);

        self.get_mut(db).arguments.new_argument(name, argument_type, var);
        var
    }

    fn set_return_type(&self, db: &mut Database, typ: TypeRef) {
        let method = self.get_mut(db);

        // If a method never returns there's no point in inlining it, because it
        // can only be called once upon which the program terminates.
        if let TypeRef::Never = typ {
            method.inline = Inline::Never;
        }

        method.return_type = typ;
    }

    fn return_type(&self, db: &Database) -> TypeRef {
        self.get(db).return_type
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Receiver {
    /// The receiver is explicit (e.g. `foo.bar()`)
    Explicit,

    /// The receiver is implicitly `self` (e.g. `bar()` and there's an instance
    /// method with that name).
    Implicit,

    /// The receiver is a class to call a static method on.
    ///
    /// This is separate from an explicit receiver as we don't need to process
    /// the receiver expression in this case.
    Class,

    /// The call is an extern call, and there's no receiver.
    Extern,
}

impl Receiver {
    pub fn without_receiver(db: &Database, method: MethodId) -> Receiver {
        if method.is_extern(db) {
            return Receiver::Extern;
        }

        method
            .receiver(db)
            .as_class(db)
            .map(|_| Receiver::Class)
            .unwrap_or(Receiver::Implicit)
    }

    pub fn with_receiver(
        db: &Database,
        receiver: TypeRef,
        method: MethodId,
    ) -> Receiver {
        if method.is_extern(db) {
            return Receiver::Extern;
        }

        receiver
            .as_class(db)
            .map(|_| Receiver::Class)
            .unwrap_or(Receiver::Explicit)
    }

    pub fn with_module(db: &Database, method: MethodId) -> Receiver {
        if method.is_extern(db) {
            return Receiver::Extern;
        }

        Receiver::Class
    }

    pub fn is_explicit(&self) -> bool {
        matches!(self, Receiver::Explicit)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallInfo {
    pub id: MethodId,
    pub receiver: Receiver,
    pub returns: TypeRef,
    pub dynamic: bool,
    pub type_arguments: TypeArguments,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClosureCallInfo {
    pub id: ClosureId,
    pub returns: TypeRef,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntrinsicCall {
    pub id: Intrinsic,
    pub returns: TypeRef,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldInfo {
    pub class: ClassId,
    pub id: FieldId,
    pub variable_type: TypeRef,
    pub as_pointer: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassInstanceInfo {
    pub class_id: ClassId,
    pub resolved_type: TypeRef,
    pub fields: Vec<(FieldId, TypeRef)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallKind {
    Unknown,
    Call(CallInfo),
    CallClosure(ClosureCallInfo),
    GetField(FieldInfo),
    SetField(FieldInfo),
    GetConstant(ConstantId),
    ReadPointer(TypeRef),
    WritePointer,
    ClassInstance(ClassInstanceInfo),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IdentifierKind {
    Unknown,
    Variable(VariableId),
    Method(CallInfo),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConstantKind {
    Unknown,
    Constant(ConstantId),
    Method(CallInfo),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConstantPatternKind {
    Unknown,
    Constructor(ConstructorId),
    String(ConstantId),
    Int(ConstantId),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ThrowKind {
    Unknown,
    Infer(TypePlaceholderId),
    Option(TypeRef),
    Result(TypeRef, TypeRef),
}

impl ThrowKind {
    pub fn throw_type_name(self, db: &Database, ok: TypeRef) -> String {
        match self {
            ThrowKind::Option(_) => {
                format!("Option[{}]", format::format_type(db, ok))
            }
            ThrowKind::Result(_, err) => {
                format!(
                    "Result[{}, {}]",
                    format::format_type(db, ok),
                    format::format_type(db, err)
                )
            }
            _ => "?".to_string(),
        }
    }

    pub fn as_uni(self, db: &Database) -> ThrowKind {
        match self {
            ThrowKind::Result(ok, err) if err.is_owned(db) => {
                ThrowKind::Result(ok, err.as_uni(db))
            }
            kind => kind,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum Symbol {
    Class(ClassId),
    Trait(TraitId),
    Module(ModuleId),
    TypeParameter(TypeParameterId),
    Constant(ConstantId),
    Method(MethodId),
}

impl Symbol {
    pub fn is_public(self, db: &Database) -> bool {
        match self {
            Symbol::Method(id) => id.is_public(db),
            Symbol::Class(id) => id.is_public(db),
            Symbol::Trait(id) => id.is_public(db),
            Symbol::Constant(id) => id.is_public(db),
            _ => true,
        }
    }

    pub fn is_private(self, db: &Database) -> bool {
        !self.is_public(db)
    }

    pub fn is_visible_to(self, db: &Database, module: ModuleId) -> bool {
        if self.is_public(db) {
            return true;
        }

        let mod_id = match self {
            Symbol::Method(id) => id.module(db),
            Symbol::Class(id) => id.module(db),
            Symbol::Trait(id) => id.module(db),
            Symbol::Constant(id) => id.module(db),
            _ => return true,
        };

        mod_id.has_same_root_namespace(db, module)
    }
}

struct ModuleSymbol {
    symbol: Symbol,
    used: bool,
}

/// An Inko module.
pub struct Module {
    name: ModuleName,

    /// The source documentation of this module.
    documentation: String,

    /// The name of this module to use when generating method symbol names.
    ///
    /// The compiler may generate new modules with generated names. This field
    /// is used when generating symbol names for methods, such that debug info
    /// uses a human readable name instead of the generated one.
    method_symbol_name: ModuleName,

    /// The ID of the class that's generated for the modle (containing the
    /// static methods).
    class: ClassId,

    /// The path to the source file of the module.
    file: PathBuf,

    /// The constants defined in this module.
    constants: Vec<ConstantId>,

    /// The symbols defined and imported into this module.
    symbols: HashMap<String, ModuleSymbol>,

    /// The external methods defined in this module.
    extern_methods: HashMap<String, MethodId>,
}

impl Module {
    pub fn alloc(
        db: &mut Database,
        name: ModuleName,
        file: PathBuf,
    ) -> ModuleId {
        assert!(db.modules.len() < u32::MAX as usize);

        let id = ModuleId(db.modules.len() as u32);
        let class_id = Class::alloc(
            db,
            name.to_string(),
            ClassKind::Module,
            Visibility::Private,
            id,
            Location::default(),
        );

        db.module_mapping.insert(name.to_string(), id);
        db.modules.push(Module {
            name: name.clone(),
            documentation: String::new(),
            method_symbol_name: name,
            class: class_id,
            file,
            constants: Vec::new(),
            symbols: HashMap::default(),
            extern_methods: HashMap::new(),
        });
        id
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct ModuleId(pub u32);

impl ModuleId {
    pub fn name(self, db: &Database) -> &ModuleName {
        &self.get(db).name
    }

    pub fn documentation(self, db: &Database) -> &String {
        &self.get(db).documentation
    }

    pub fn set_documentation(self, db: &mut Database, value: String) {
        self.get_mut(db).documentation = value;
    }

    pub fn constants(self, db: &Database) -> &Vec<ConstantId> {
        &self.get(db).constants
    }

    pub fn method_symbol_name(self, db: &Database) -> &ModuleName {
        &self.get(db).method_symbol_name
    }

    pub fn set_method_symbol_name(self, db: &mut Database, name: ModuleName) {
        self.get_mut(db).method_symbol_name = name;
    }

    pub fn file(self, db: &Database) -> PathBuf {
        self.get(db).file.clone()
    }

    pub fn use_symbol(self, db: &mut Database, name: &str) -> Option<Symbol> {
        let module = self.get_mut(db);

        if let Some(sym) = module.symbols.get_mut(name) {
            sym.used = true;
            Some(sym.symbol)
        } else {
            None
        }
    }

    fn symbol(self, db: &Database, name: &str) -> Option<Symbol> {
        self.get(db).symbols.get(name).map(|v| v.symbol)
    }

    pub fn symbol_is_used(self, db: &Database, name: &str) -> bool {
        self.get(db).symbols.get(name).map_or(false, |v| v.used)
    }

    pub fn symbols(self, db: &Database) -> Vec<(String, Symbol)> {
        self.get(db)
            .symbols
            .iter()
            .map(|(name, value)| (name.clone(), value.symbol))
            .collect()
    }

    pub fn symbol_exists(self, db: &Database, name: &str) -> bool {
        self.get(db).symbols.contains_key(name)
    }

    pub fn import_symbol(
        self,
        db: &mut Database,
        name: &str,
    ) -> Option<Symbol> {
        let symbol = self.use_symbol(db, name)?;
        let module_id = match symbol {
            Symbol::Class(id) => id.module(db),
            Symbol::Trait(id) => id.module(db),
            Symbol::Constant(id) => id.module(db),
            Symbol::Method(id) => id.module(db),
            Symbol::Module(id) => id,
            // Type parameters can't be imported.
            Symbol::TypeParameter(_) => return None,
        };

        if self == module_id {
            Some(symbol)
        } else {
            None
        }
    }

    pub fn new_symbol(self, db: &mut Database, name: String, symbol: Symbol) {
        self.get_mut(db)
            .symbols
            .insert(name, ModuleSymbol { symbol, used: false });
    }

    pub fn method(self, db: &Database, name: &str) -> Option<MethodId> {
        self.get(db).class.method(db, name)
    }

    pub fn methods(self, db: &Database) -> Vec<MethodId> {
        self.get(db).class.methods(db)
    }

    pub fn classes(self, db: &Database) -> Vec<ClassId> {
        self.get(db)
            .symbols
            .iter()
            .filter_map(|(name, s)| match s.symbol {
                // Generated symbol names start with "$", which we never want to
                // include.
                Symbol::Class(id)
                    if id.module(db) == self && !name.starts_with('$') =>
                {
                    Some(id)
                }
                _ => None,
            })
            .collect()
    }

    pub fn traits(self, db: &Database) -> Vec<TraitId> {
        self.get(db)
            .symbols
            .values()
            .filter_map(|s| match s.symbol {
                Symbol::Trait(id) if id.module(db) == self => Some(id),
                _ => None,
            })
            .collect()
    }

    pub fn add_method(self, db: &mut Database, name: String, method: MethodId) {
        self.get(db).class.add_method(db, name, method);
    }

    pub fn add_extern_method(self, db: &mut Database, method: MethodId) {
        let name = method.name(db).clone();

        self.get_mut(db).extern_methods.insert(name, method);
    }

    pub fn extern_method(self, db: &Database, name: &str) -> Option<MethodId> {
        self.get(db).extern_methods.get(name).cloned()
    }

    pub fn extern_methods(self, db: &Database) -> &HashMap<String, MethodId> {
        &self.get(db).extern_methods
    }

    pub fn is_std(self, db: &Database) -> bool {
        self.get(db).name.is_std()
    }

    pub fn class(self, db: &Database) -> ClassId {
        self.get(db).class
    }

    fn has_same_root_namespace(self, db: &Database, other: ModuleId) -> bool {
        let ours = self.name(db);
        let theirs = other.name(db);

        if ours.head() == theirs.head() {
            return true;
        }

        if !theirs.is_root() {
            return false;
        }

        // This allow the top-level test module `test_foo` to import private
        // symbols from the top-level module `foo`, but not the other way
        // around.
        theirs
            .as_str()
            .strip_prefix("test_")
            .map_or(false, |name| ours.head() == name)
    }

    fn get(self, db: &Database) -> &Module {
        &db.modules[self.0 as usize]
    }

    fn get_mut(self, db: &mut Database) -> &mut Module {
        &mut db.modules[self.0 as usize]
    }
}

/// A local variable.
pub struct Variable {
    /// The user-defined name of the variable.
    name: String,

    /// The type of the constant's value.
    value_type: TypeRef,

    /// A flat set to `true` if the variable can be assigned a new value.
    mutable: bool,

    /// The location of the variable.
    location: Location,
}

impl Variable {
    pub fn alloc(
        db: &mut Database,
        name: String,
        value_type: TypeRef,
        mutable: bool,
        location: Location,
    ) -> VariableId {
        let id = VariableId(db.variables.len());

        db.variables.push(Self { name, value_type, mutable, location });
        id
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Ord, PartialOrd, Hash)]
pub struct VariableId(pub usize);

impl VariableId {
    pub fn name(self, db: &Database) -> &String {
        &self.get(db).name
    }

    pub fn value_type(self, db: &Database) -> TypeRef {
        self.get(db).value_type
    }

    pub fn set_value_type(self, db: &mut Database, typ: TypeRef) {
        self.get_mut(db).value_type = typ;
    }

    pub fn is_mutable(self, db: &Database) -> bool {
        self.get(db).mutable
    }

    pub fn location(self, db: &Database) -> Location {
        self.get(db).location
    }

    fn get(self, db: &Database) -> &Variable {
        &db.variables[self.0]
    }

    fn get_mut(self, db: &mut Database) -> &mut Variable {
        &mut db.variables[self.0]
    }
}

/// A constant.
///
/// Unlike variables, constants can't be assigned new values. They are also
/// limited to values of a select few types.
pub struct Constant {
    id: u16,
    module: ModuleId,
    location: Location,
    name: String,
    documentation: String,
    value_type: TypeRef,
    visibility: Visibility,
}

impl Constant {
    pub fn alloc(
        db: &mut Database,
        module: ModuleId,
        location: Location,
        name: String,
        visibility: Visibility,
        value_type: TypeRef,
    ) -> ConstantId {
        let global_id = db.constants.len();
        let local_id = module.get(db).constants.len();

        assert!(local_id <= u16::MAX as usize);

        let constant = Constant {
            id: local_id as u16,
            module,
            location,
            name: name.clone(),
            documentation: String::new(),
            value_type,
            visibility,
        };

        let const_id = ConstantId(global_id);

        module.get_mut(db).constants.push(const_id);
        module.new_symbol(db, name, Symbol::Constant(const_id));
        db.constants.push(constant);
        const_id
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct ConstantId(pub usize);

impl ConstantId {
    pub fn module_local_id(self, db: &Database) -> u16 {
        self.get(db).id
    }

    pub fn location(self, db: &Database) -> Location {
        self.get(db).location
    }

    pub fn name(self, db: &Database) -> &String {
        &self.get(db).name
    }

    pub fn module(self, db: &Database) -> ModuleId {
        self.get(db).module
    }

    pub fn set_value_type(self, db: &mut Database, value_type: TypeRef) {
        self.get_mut(db).value_type = value_type;
    }

    pub fn value_type(self, db: &Database) -> TypeRef {
        self.get(db).value_type
    }

    pub fn is_public(self, db: &Database) -> bool {
        self.get(db).visibility == Visibility::Public
    }

    pub fn set_documentation(self, db: &mut Database, value: String) {
        self.get_mut(db).documentation = value;
    }

    pub fn documentation(self, db: &Database) -> &String {
        &self.get(db).documentation
    }

    fn get(self, db: &Database) -> &Constant {
        &db.constants[self.0]
    }

    fn get_mut(self, db: &mut Database) -> &mut Constant {
        &mut db.constants[self.0]
    }
}

/// An anonymous function that can optionally capture outer variables.
///
/// Unlike methods, closures don't support type parameters. This makes it easier
/// to implement them, and generic closures aren't that useful to begin with. Of
/// course closures _can_ refer to type parameters defined in the surrounding
/// method or type.
#[derive(Clone)]
pub struct Closure {
    moving: bool,

    /// The variables captured by this closure, and the types the variables are
    /// captured as.
    captured: HashSet<(VariableId, TypeRef)>,

    /// The type of `self` as captured by the closure.
    captured_self_type: Option<TypeRef>,
    arguments: Arguments,
    return_type: TypeRef,
}

impl Closure {
    pub fn alloc(db: &mut Database, moving: bool) -> ClosureId {
        let closure = Closure::new(moving);

        Self::add(db, closure)
    }

    pub(crate) fn add(db: &mut Database, closure: Closure) -> ClosureId {
        let id = db.closures.len();

        db.closures.push(closure);
        ClosureId(id)
    }

    fn new(moving: bool) -> Self {
        Self {
            moving,
            captured_self_type: None,
            captured: HashSet::new(),
            arguments: Arguments::new(),
            return_type: TypeRef::Unknown,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct ClosureId(pub usize);

impl ClosureId {
    pub fn number_of_arguments(self, db: &Database) -> usize {
        self.get(db).arguments.len()
    }

    pub fn positional_argument_input_type(
        self,
        db: &Database,
        index: usize,
    ) -> Option<TypeRef> {
        self.get(db)
            .arguments
            .mapping
            .get_index(index)
            .map(|(_, a)| a.value_type)
    }

    pub fn new_anonymous_argument(
        self,
        db: &mut Database,
        value_type: TypeRef,
    ) {
        let closure = self.get_mut(db);

        // Anonymous arguments can never be used, so the variable ID is never
        // used. As such we just set it to ID 0 so we don't need to wrap it in
        // an `Option` type.
        let var = VariableId(0);
        let name = format!("_arg{}", closure.arguments.len());

        closure.arguments.new_argument(name, value_type, var);
    }

    pub fn is_moving(self, db: &Database) -> bool {
        self.get(db).moving
    }

    pub fn set_captured_self_type(
        self,
        db: &mut Database,
        value_type: TypeRef,
    ) {
        self.get_mut(db).captured_self_type = Some(value_type);
    }

    pub fn captured_self_type(self, db: &Database) -> Option<TypeRef> {
        self.get(db).captured_self_type
    }

    pub fn add_capture(
        self,
        db: &mut Database,
        variable: VariableId,
        captured_as: TypeRef,
    ) {
        self.get_mut(db).captured.insert((variable, captured_as));
    }

    pub fn captured(self, db: &Database) -> Vec<(VariableId, TypeRef)> {
        self.get(db).captured.iter().cloned().collect()
    }

    pub fn arguments(self, db: &Database) -> Vec<Argument> {
        self.get(db).arguments.mapping.values().cloned().collect()
    }

    pub fn can_infer_as_uni(self, db: &Database) -> bool {
        let closure = self.get(db);

        if !closure.captured.iter().all(|(_, typ)| typ.is_sendable(db)) {
            return false;
        }

        match closure.captured_self_type {
            Some(typ) if typ.is_stack_allocated(db) => true,
            Some(_) => false,
            _ => true,
        }
    }

    pub(crate) fn get(self, db: &Database) -> &Closure {
        &db.closures[self.0]
    }

    fn get_mut(self, db: &mut Database) -> &mut Closure {
        &mut db.closures[self.0]
    }
}

impl Block for ClosureId {
    fn new_argument(
        &self,
        db: &mut Database,
        name: String,
        variable_type: TypeRef,
        argument_type: TypeRef,
        location: Location,
    ) -> VariableId {
        let var =
            Variable::alloc(db, name.clone(), variable_type, false, location);

        self.get_mut(db).arguments.new_argument(name, argument_type, var);
        var
    }

    fn set_return_type(&self, db: &mut Database, typ: TypeRef) {
        self.get_mut(db).return_type = typ;
    }

    fn return_type(&self, db: &Database) -> TypeRef {
        self.get(db).return_type
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum Sign {
    Signed,
    Unsigned,
}

impl Sign {
    pub fn is_signed(self) -> bool {
        matches!(self, Sign::Signed)
    }
}

/// A type describing the "shape" of a type, which describes its size on the
/// stack, how to create aliases, etc.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum Shape {
    /// An owned value addressed through a pointer.
    Owned,

    /// A mutable reference to a value.
    Mut,

    /// An immutable reference to a value.
    Ref,

    /// A 64-bits unboxed integer.
    ///
    /// The arguments are:
    ///
    /// 1. The size in bits
    /// 2. The sign of the integer
    Int(u32, Sign),

    /// A 64-bits unboxed float.
    ///
    /// The argument is the size in bits.
    Float(u32),

    /// A boolean.
    Boolean,

    /// A string using atomic reference counting.
    String,

    /// The nil singleton.
    Nil,

    /// An owned value that uses atomic reference counting.
    Atomic,

    /// A raw C pointer.
    ///
    /// This is a dedicated shape because:
    ///
    /// - Pointers aren't necessarily integers (e.g. with CHERI)
    /// - It better signals the purpose is for raw pointers and not random
    ///   integers
    Pointer,

    /// A shape for a specific stack allocated type.
    ///
    /// While comparing `ClassInstance` values for equality is normally not
    /// reliable (as different occurrences of the same generic type use
    /// different type argument IDs), this is made reliable by interning
    /// structurually different but semantically equivalent `ClassInstance`
    /// values into a single common `ClassInstance`, such that the comparison
    /// _is_ reliable.
    Stack(ClassInstance),
}

impl Shape {
    pub fn int() -> Shape {
        Shape::Int(64, Sign::Signed)
    }

    pub fn float() -> Shape {
        Shape::Float(64)
    }

    pub fn is_foreign(self) -> bool {
        match self {
            Shape::Int(64, Sign::Signed) => false,
            Shape::Int(_, Sign::Unsigned) => true,
            Shape::Int(_, Sign::Signed) => true,
            Shape::Float(32) => true,
            _ => false,
        }
    }
}

/// A reference to a type.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum TypeRef {
    /// An owned value subject to move semantics.
    Owned(TypeId),

    /// An owned value subject to move semantics, and doesn't allow aliasing.
    Uni(TypeId),

    /// An immutable reference to a type.
    Ref(TypeId),

    /// An immutable, temporary and unique reference.
    UniRef(TypeId),

    /// A mutable reference to a type.
    Mut(TypeId),

    /// A mutable, temporary and unique reference.
    UniMut(TypeId),

    /// A type of which the ownership can be anything.
    ///
    /// This constructor is only used with type parameters. We wrap a TypeId
    /// here so we can reuse various functions more easily, such as those used
    /// for type-checking; instead of having to special-case this constructor.
    Any(TypeId),

    /// A type that signals something never happens.
    ///
    /// When used as a return type, it means a method never returns.
    Never,

    /// A value indicating a typing error.
    ///
    /// This type is produced whenever a type couldn't be produced, for example
    /// by calling a method on an undefined variable.
    Error,

    /// The type is not yet known.
    ///
    /// This is the default state for a type.
    Unknown,

    /// A placeholder for a yet-to-infer type.
    Placeholder(TypePlaceholderId),

    /// A pointer to a value.
    Pointer(TypeId),
}

impl TypeRef {
    pub fn nil() -> TypeRef {
        TypeRef::Owned(TypeId::ClassInstance(ClassInstance::new(ClassId(
            NIL_ID,
        ))))
    }

    pub fn boolean() -> TypeRef {
        TypeRef::Owned(TypeId::ClassInstance(ClassInstance::new(ClassId(
            BOOL_ID,
        ))))
    }

    pub fn int() -> TypeRef {
        TypeRef::Owned(TypeId::ClassInstance(
            ClassInstance::new(ClassId::int()),
        ))
    }

    pub fn float() -> TypeRef {
        TypeRef::Owned(TypeId::ClassInstance(ClassInstance::new(
            ClassId::float(),
        )))
    }

    pub fn string() -> TypeRef {
        TypeRef::Owned(TypeId::ClassInstance(ClassInstance::new(
            ClassId::string(),
        )))
    }

    pub fn byte_array() -> TypeRef {
        TypeRef::Owned(TypeId::ClassInstance(ClassInstance::new(
            ClassId::byte_array(),
        )))
    }

    pub fn int_with_sign(size: u32, sign: Sign) -> TypeRef {
        match sign {
            Sign::Signed if size == 64 => TypeRef::int(),
            Sign::Signed => TypeRef::foreign_signed_int(size),
            _ => TypeRef::foreign_unsigned_int(size),
        }
    }

    pub fn float_with_size(size: u32) -> TypeRef {
        if size == 64 {
            TypeRef::float()
        } else {
            TypeRef::foreign_float(size)
        }
    }

    pub fn foreign_signed_int(size: u32) -> TypeRef {
        TypeRef::Owned(TypeId::Foreign(ForeignType::Int(size, Sign::Signed)))
    }

    pub fn foreign_unsigned_int(size: u32) -> TypeRef {
        TypeRef::Owned(TypeId::Foreign(ForeignType::Int(size, Sign::Unsigned)))
    }

    pub fn foreign_float(size: u32) -> TypeRef {
        TypeRef::Owned(TypeId::Foreign(ForeignType::Float(size)))
    }

    pub fn pointer(of: TypeId) -> TypeRef {
        TypeRef::Pointer(of)
    }

    pub fn module(id: ModuleId) -> TypeRef {
        TypeRef::Owned(TypeId::Module(id))
    }

    pub fn placeholder(
        db: &mut Database,
        required: Option<TypeParameterId>,
    ) -> TypeRef {
        TypeRef::Placeholder(TypePlaceholder::alloc(db, required))
    }

    pub fn type_id(self, db: &Database) -> Result<TypeId, TypeRef> {
        match self {
            TypeRef::Pointer(id)
            | TypeRef::Owned(id)
            | TypeRef::Uni(id)
            | TypeRef::Ref(id)
            | TypeRef::Mut(id)
            | TypeRef::UniRef(id)
            | TypeRef::UniMut(id)
            | TypeRef::Any(id) => Ok(id),
            TypeRef::Placeholder(id) => {
                id.value(db).ok_or(self).and_then(|t| t.type_id(db))
            }
            _ => Err(self),
        }
    }

    pub fn closure_id(self, db: &Database) -> Option<ClosureId> {
        if let Ok(TypeId::Closure(id)) = self.type_id(db) {
            Some(id)
        } else {
            None
        }
    }

    pub fn is_never(self, db: &Database) -> bool {
        match self {
            TypeRef::Never => true,
            TypeRef::Placeholder(id) => {
                id.value(db).map_or(false, |v| v.is_never(db))
            }
            _ => false,
        }
    }

    pub fn allow_in_array(self, db: &Database) -> bool {
        match self {
            TypeRef::UniRef(_) | TypeRef::UniMut(_) => false,
            TypeRef::Placeholder(id) => {
                id.value(db).map_or(true, |v| v.allow_in_array(db))
            }
            _ => !self.is_foreign_type(db),
        }
    }

    pub fn is_foreign_type(self, db: &Database) -> bool {
        match self {
            TypeRef::Owned(TypeId::ClassInstance(ins))
                if ins.instance_of.kind(db).is_extern() =>
            {
                true
            }
            TypeRef::Owned(TypeId::Foreign(_)) => true,
            TypeRef::Pointer(_) => true,
            TypeRef::Placeholder(id) => {
                id.value(db).map_or(false, |v| v.is_foreign_type(db))
            }
            _ => false,
        }
    }

    pub fn is_extern_instance(self, db: &Database) -> bool {
        match self {
            TypeRef::Owned(TypeId::ClassInstance(ins))
            | TypeRef::Uni(TypeId::ClassInstance(ins)) => {
                ins.instance_of().kind(db).is_extern()
            }
            _ => false,
        }
    }

    pub fn is_pointer(self, db: &Database) -> bool {
        match self {
            TypeRef::Pointer(_) => true,
            TypeRef::Placeholder(id) => {
                id.value(db).map_or(false, |v| v.is_pointer(db))
            }
            _ => false,
        }
    }

    pub fn is_error(self, db: &Database) -> bool {
        match self {
            TypeRef::Error => true,
            TypeRef::Placeholder(id) => {
                id.value(db).map_or(false, |v| v.is_error(db))
            }
            _ => false,
        }
    }

    pub fn is_present(self, db: &Database) -> bool {
        match self {
            TypeRef::Never => false,
            TypeRef::Placeholder(id) => {
                id.value(db).map_or(false, |v| v.is_present(db))
            }
            _ => true,
        }
    }

    pub fn is_owned_or_uni(self, db: &Database) -> bool {
        match self {
            TypeRef::Owned(_) | TypeRef::Uni(_) | TypeRef::Any(_) => true,
            TypeRef::Placeholder(id) => {
                id.value(db).map_or(false, |v| v.is_owned_or_uni(db))
            }
            _ => false,
        }
    }

    pub fn is_owned(self, db: &Database) -> bool {
        match self {
            TypeRef::Owned(_) => true,
            TypeRef::Placeholder(id) => {
                id.value(db).map_or(false, |v| v.is_owned(db))
            }
            _ => false,
        }
    }

    pub fn is_type_parameter(self, db: &Database) -> bool {
        match self {
            TypeRef::Owned(
                TypeId::TypeParameter(_)
                | TypeId::RigidTypeParameter(_)
                | TypeId::AtomicTypeParameter(_),
            )
            | TypeRef::Uni(
                TypeId::TypeParameter(_)
                | TypeId::RigidTypeParameter(_)
                | TypeId::AtomicTypeParameter(_),
            )
            | TypeRef::Ref(
                TypeId::TypeParameter(_)
                | TypeId::RigidTypeParameter(_)
                | TypeId::AtomicTypeParameter(_),
            )
            | TypeRef::Mut(
                TypeId::TypeParameter(_)
                | TypeId::RigidTypeParameter(_)
                | TypeId::AtomicTypeParameter(_),
            )
            | TypeRef::Any(
                TypeId::TypeParameter(_)
                | TypeId::RigidTypeParameter(_)
                | TypeId::AtomicTypeParameter(_),
            )
            | TypeRef::UniRef(
                TypeId::TypeParameter(_)
                | TypeId::RigidTypeParameter(_)
                | TypeId::AtomicTypeParameter(_),
            )
            | TypeRef::UniMut(
                TypeId::TypeParameter(_)
                | TypeId::RigidTypeParameter(_)
                | TypeId::AtomicTypeParameter(_),
            ) => true,
            TypeRef::Placeholder(id) => {
                id.value(db).map_or(false, |v| v.is_type_parameter(db))
            }
            _ => false,
        }
    }

    pub fn is_rigid_type_parameter(self, db: &Database) -> bool {
        matches!(self.type_id(db), Ok(TypeId::RigidTypeParameter(_)))
    }

    pub fn is_trait_instance(self, db: &Database) -> bool {
        match self {
            TypeRef::Owned(TypeId::TraitInstance(_))
            | TypeRef::Uni(TypeId::TraitInstance(_))
            | TypeRef::Ref(TypeId::TraitInstance(_))
            | TypeRef::Mut(TypeId::TraitInstance(_))
            | TypeRef::UniRef(TypeId::TraitInstance(_))
            | TypeRef::UniMut(TypeId::TraitInstance(_)) => true,
            TypeRef::Placeholder(id) => {
                id.value(db).map_or(false, |v| v.is_trait_instance(db))
            }
            _ => false,
        }
    }

    pub fn type_arguments(self, db: &Database) -> TypeArguments {
        match self.type_id(db) {
            Ok(TypeId::TraitInstance(ins))
                if ins.instance_of.is_generic(db) =>
            {
                ins.type_arguments(db).unwrap().clone()
            }
            Ok(TypeId::ClassInstance(ins))
                if ins.instance_of.is_generic(db) =>
            {
                ins.type_arguments(db).unwrap().clone()
            }
            Ok(TypeId::TypeParameter(id) | TypeId::RigidTypeParameter(id)) => {
                id.requirements(db)
                    .into_iter()
                    .filter(|r| r.instance_of.is_generic(db))
                    .fold(TypeArguments::new(), |mut targs, req| {
                        req.type_arguments(db).unwrap().copy_into(&mut targs);
                        req.instance_of()
                            .get(db)
                            .inherited_type_arguments
                            .copy_into(&mut targs);

                        targs
                    })
            }
            _ => TypeArguments::new(),
        }
    }

    pub fn is_uni(self, db: &Database) -> bool {
        match self {
            TypeRef::Uni(_) => true,
            TypeRef::Placeholder(id) => {
                id.value(db).map_or(false, |v| v.is_uni(db))
            }
            _ => false,
        }
    }

    pub fn require_sendable_arguments(self, db: &Database) -> bool {
        match self {
            TypeRef::Uni(_) | TypeRef::UniRef(_) | TypeRef::UniMut(_) => true,
            TypeRef::Placeholder(id) => {
                id.value(db).map_or(false, |v| v.require_sendable_arguments(db))
            }
            _ => false,
        }
    }

    pub fn is_sendable_ref(self, db: &Database) -> bool {
        match self {
            TypeRef::Ref(_) | TypeRef::UniRef(_) => true,
            TypeRef::Placeholder(id) => {
                id.value(db).map_or(false, |v| v.is_sendable_ref(db))
            }
            _ => false,
        }
    }

    pub fn is_ref(self, db: &Database) -> bool {
        match self {
            TypeRef::Ref(_) => true,
            TypeRef::Placeholder(id) => {
                id.value(db).map_or(false, |v| v.is_ref(db))
            }
            _ => false,
        }
    }

    pub fn is_mut(self, db: &Database) -> bool {
        match self {
            TypeRef::Mut(_) => true,
            TypeRef::Placeholder(id) => {
                id.value(db).map_or(false, |v| v.is_ref(db))
            }
            _ => false,
        }
    }

    pub fn is_ref_or_mut(self, db: &Database) -> bool {
        match self {
            TypeRef::Mut(_) | TypeRef::Ref(_) => true,
            TypeRef::Placeholder(id) => {
                id.value(db).map_or(false, |v| v.is_ref_or_mut(db))
            }
            _ => false,
        }
    }

    pub fn has_ownership(self, db: &Database) -> bool {
        match self {
            TypeRef::Owned(_)
            | TypeRef::Uni(_)
            | TypeRef::Ref(_)
            | TypeRef::Mut(_)
            | TypeRef::Pointer(_) => true,
            TypeRef::Placeholder(id) => {
                id.value(db).map_or(false, |v| v.has_ownership(db))
            }
            _ => false,
        }
    }

    pub fn use_reference_counting(self, db: &Database) -> bool {
        match self {
            TypeRef::Ref(_)
            | TypeRef::Mut(_)
            | TypeRef::UniRef(_)
            | TypeRef::UniMut(_) => true,
            TypeRef::Placeholder(id) => {
                id.value(db).map_or(false, |v| v.use_reference_counting(db))
            }
            _ => false,
        }
    }

    pub fn use_atomic_reference_counting(self, db: &Database) -> bool {
        self.class_id(db).map_or(false, |id| id.is_atomic(db))
    }

    pub fn is_bool(self, db: &Database) -> bool {
        self.is_instance_of(db, ClassId::boolean())
    }

    pub fn is_int(self, db: &Database) -> bool {
        self.is_instance_of(db, ClassId::int())
    }

    pub fn is_string(self, db: &Database) -> bool {
        self.is_instance_of(db, ClassId::string())
    }

    pub fn is_nil(self, db: &Database) -> bool {
        self.is_instance_of(db, ClassId::nil())
    }

    pub fn allow_moving(self, db: &Database) -> bool {
        match self {
            TypeRef::Owned(_) | TypeRef::Uni(_) => true,
            TypeRef::UniRef(TypeId::ClassInstance(i))
            | TypeRef::UniMut(TypeId::ClassInstance(i)) => {
                i.instance_of.is_stack_allocated(db)
            }
            TypeRef::Placeholder(id) => {
                id.value(db).map_or(false, |v| v.allow_moving(db))
            }
            _ => false,
        }
    }

    pub fn allow_mutating(self, db: &Database) -> bool {
        match self {
            TypeRef::Owned(TypeId::ClassInstance(ins))
            | TypeRef::Mut(TypeId::ClassInstance(ins)) => {
                ins.instance_of.allow_mutating(db)
            }
            TypeRef::Owned(_)
            | TypeRef::Uni(_)
            | TypeRef::Mut(_)
            | TypeRef::UniMut(_)
            | TypeRef::Pointer(_) => true,
            TypeRef::Any(
                TypeId::TypeParameter(id) | TypeId::RigidTypeParameter(id),
            ) => id.is_mutable(db),
            TypeRef::Ref(TypeId::ClassInstance(ins)) => {
                ins.instance_of.is_value_type(db)
                    && !ins.instance_of().kind(db).is_async()
            }
            TypeRef::Placeholder(id) => {
                id.value(db).map_or(false, |v| v.allow_mutating(db))
            }
            _ => false,
        }
    }

    pub fn as_class_instance_for_pattern_matching(
        self,
        db: &Database,
    ) -> Option<ClassInstance> {
        match self {
            TypeRef::Owned(TypeId::ClassInstance(ins))
            | TypeRef::Uni(TypeId::ClassInstance(ins))
            | TypeRef::Mut(TypeId::ClassInstance(ins))
            | TypeRef::Ref(TypeId::ClassInstance(ins))
                if ins.instance_of.kind(db).allow_pattern_matching() =>
            {
                Some(ins)
            }
            TypeRef::Placeholder(id) => id
                .value(db)
                .and_then(|v| v.as_class_instance_for_pattern_matching(db)),
            _ => None,
        }
    }

    pub fn is_uni_ref(self, db: &Database) -> bool {
        match self {
            TypeRef::UniRef(_) | TypeRef::UniMut(_) => true,
            TypeRef::Placeholder(id) => {
                id.value(db).map_or(false, |v| v.is_uni_ref(db))
            }
            _ => false,
        }
    }

    pub fn is_sendable(self, db: &Database) -> bool {
        match self {
            TypeRef::Uni(_) | TypeRef::Never | TypeRef::Error => true,
            TypeRef::Owned(TypeId::Closure(id)) => id.can_infer_as_uni(db),
            TypeRef::Placeholder(id) => {
                id.value(db).map_or(true, |v| v.is_sendable(db))
            }
            _ => self.is_value_type(db),
        }
    }

    pub fn is_sendable_output(self, db: &Database) -> bool {
        match self {
            TypeRef::Uni(_) | TypeRef::Never | TypeRef::Error => true,
            TypeRef::Owned(TypeId::ClassInstance(id)) => {
                let class = id.instance_of;

                if class.is_generic(db)
                    && !id
                        .type_arguments(db)
                        .unwrap()
                        .iter()
                        .all(|(_, v)| v.is_sendable_output(db))
                {
                    return false;
                }

                class
                    .fields(db)
                    .into_iter()
                    .all(|f| f.value_type(db).is_sendable_output(db))
            }
            TypeRef::Placeholder(id) => {
                id.value(db).map_or(true, |v| v.is_sendable_output(db))
            }
            _ => self.is_value_type(db),
        }
    }

    pub fn cast_according_to(self, db: &Database, other: TypeRef) -> Self {
        if self.is_value_type(db) {
            return if other.is_uni(db) {
                self.as_uni(db)
            } else if other.is_ref_or_mut(db) && self.is_extern_instance(db) {
                self.as_pointer(db)
            } else {
                self.as_owned(db)
            };
        }

        if other.is_ref(db) {
            self.as_ref(db)
        } else if other.is_mut(db) {
            self.as_mut(db)
        } else {
            self
        }
    }

    pub fn value_type_as_owned(self, db: &Database) -> Self {
        if self.is_value_type(db) {
            self.as_owned(db)
        } else {
            self
        }
    }

    pub fn as_ref(self, db: &Database) -> Self {
        match self {
            TypeRef::Owned(TypeId::ClassInstance(ins))
                if ins.instance_of().kind(db).is_extern() =>
            {
                TypeRef::Pointer(TypeId::ClassInstance(ins))
            }
            TypeRef::Owned(TypeId::ClassInstance(ins))
                if ins.instance_of().is_stack_allocated(db) =>
            {
                TypeRef::Owned(TypeId::ClassInstance(ins))
            }
            TypeRef::Owned(id)
            | TypeRef::Any(id)
            | TypeRef::Mut(id)
            | TypeRef::Ref(id) => match id {
                TypeId::TypeParameter(pid)
                | TypeId::RigidTypeParameter(pid)
                    if pid.is_stack_allocated(db) =>
                {
                    TypeRef::Owned(id)
                }
                _ => TypeRef::Ref(id),
            },
            TypeRef::Uni(id) | TypeRef::UniMut(id) => TypeRef::UniRef(id),
            TypeRef::Placeholder(id) => {
                if let Some(v) = id.value(db) {
                    v.as_ref(db)
                } else {
                    TypeRef::Placeholder(id.as_ref())
                }
            }
            _ => self,
        }
    }

    pub fn allow_as_ref(self, db: &Database) -> bool {
        match self {
            TypeRef::Owned(_)
            | TypeRef::Mut(_)
            | TypeRef::Ref(_)
            | TypeRef::Uni(_)
            | TypeRef::Any(_)
            | TypeRef::Error => true,
            TypeRef::Placeholder(id) => {
                id.value(db).map_or(false, |v| v.allow_as_ref(db))
            }
            _ => false,
        }
    }

    pub fn as_mut(self, db: &Database) -> Self {
        match self {
            TypeRef::Any(
                id @ TypeId::RigidTypeParameter(pid)
                | id @ TypeId::TypeParameter(pid),
            ) => {
                if pid.is_stack_allocated(db) {
                    TypeRef::Owned(id)
                } else if pid.is_mutable(db) {
                    TypeRef::Mut(id)
                } else {
                    TypeRef::Ref(id)
                }
            }
            TypeRef::Owned(
                id @ TypeId::RigidTypeParameter(pid)
                | id @ TypeId::TypeParameter(pid),
            )
            | TypeRef::Mut(
                id @ TypeId::RigidTypeParameter(pid)
                | id @ TypeId::TypeParameter(pid),
            ) => {
                if pid.is_stack_allocated(db) {
                    TypeRef::Owned(id)
                } else {
                    TypeRef::Mut(id)
                }
            }
            TypeRef::Uni(
                id @ TypeId::RigidTypeParameter(pid)
                | id @ TypeId::TypeParameter(pid),
            ) if pid.is_stack_allocated(db) => TypeRef::Owned(id),
            TypeRef::Owned(TypeId::ClassInstance(ins))
                if ins.instance_of().kind(db).is_extern() =>
            {
                TypeRef::Pointer(TypeId::ClassInstance(ins))
            }
            TypeRef::Owned(TypeId::ClassInstance(ins))
                if ins.instance_of().is_value_type(db) =>
            {
                TypeRef::Owned(TypeId::ClassInstance(ins))
            }
            TypeRef::Owned(id) => TypeRef::Mut(id),
            TypeRef::Uni(id) => TypeRef::UniMut(id),
            TypeRef::Placeholder(id) => {
                if let Some(v) = id.value(db) {
                    v.as_mut(db)
                } else {
                    TypeRef::Placeholder(id.as_mut())
                }
            }
            _ => self,
        }
    }

    pub fn force_as_mut(self, db: &Database) -> Self {
        match self {
            TypeRef::Owned(TypeId::ClassInstance(ins))
                if ins.instance_of().kind(db).is_extern() =>
            {
                TypeRef::Pointer(TypeId::ClassInstance(ins))
            }
            TypeRef::Owned(TypeId::ClassInstance(ins))
                if ins.instance_of().is_value_type(db) =>
            {
                TypeRef::Owned(TypeId::ClassInstance(ins))
            }
            TypeRef::Owned(id) | TypeRef::Any(id) | TypeRef::Mut(id) => {
                match id {
                    TypeId::TypeParameter(tid)
                    | TypeId::RigidTypeParameter(tid)
                        if tid.is_stack_allocated(db) =>
                    {
                        TypeRef::Owned(id)
                    }
                    _ => TypeRef::Mut(id),
                }
            }
            TypeRef::Uni(id) => match id {
                TypeId::TypeParameter(tid)
                | TypeId::RigidTypeParameter(tid)
                    if tid.is_stack_allocated(db) =>
                {
                    TypeRef::Owned(id)
                }
                _ => TypeRef::UniMut(id),
            },
            TypeRef::Placeholder(id) => {
                if let Some(v) = id.value(db) {
                    v.force_as_mut(db)
                } else {
                    TypeRef::Placeholder(id.as_mut())
                }
            }
            _ => self,
        }
    }

    pub fn as_pointer(self, db: &Database) -> TypeRef {
        match self {
            TypeRef::Owned(id)
            | TypeRef::Uni(id)
            | TypeRef::Any(id)
            | TypeRef::Mut(id)
            | TypeRef::Ref(id) => TypeRef::Pointer(id),
            TypeRef::Placeholder(id) => {
                if let Some(v) = id.value(db) {
                    v.as_pointer(db)
                } else {
                    TypeRef::Placeholder(id.as_pointer())
                }
            }
            _ => self,
        }
    }

    pub fn as_uni_borrow(self, db: &Database) -> Self {
        // Value types can always be exposed to recover blocks, as we can simply
        // copy them upon moving them around.
        if self.is_value_type(db) {
            return self;
        }

        match self {
            TypeRef::Owned(id) | TypeRef::Mut(id) => TypeRef::UniMut(id),
            TypeRef::Ref(id) => TypeRef::UniRef(id),
            TypeRef::Placeholder(id) => {
                if let Some(v) = id.value(db) {
                    v.as_uni_borrow(db)
                } else {
                    TypeRef::Placeholder(id.as_uni_mut())
                }
            }
            _ => self,
        }
    }

    pub fn as_uni_ref(self, db: &Database) -> Self {
        match self {
            TypeRef::Owned(id)
            | TypeRef::Any(id)
            | TypeRef::Uni(id)
            | TypeRef::Mut(id)
            | TypeRef::Ref(id)
            | TypeRef::UniRef(id)
            | TypeRef::UniMut(id) => TypeRef::UniRef(id),
            TypeRef::Placeholder(id) => {
                if let Some(v) = id.value(db) {
                    v.as_uni_ref(db)
                } else {
                    TypeRef::Placeholder(id.as_uni_ref())
                }
            }
            _ => self,
        }
    }

    pub fn as_uni_mut(self, db: &Database) -> Self {
        match self {
            TypeRef::Owned(id)
            | TypeRef::Uni(id)
            | TypeRef::Mut(id)
            | TypeRef::UniMut(id) => TypeRef::UniMut(id),
            TypeRef::Ref(id) | TypeRef::Any(id) | TypeRef::UniRef(id) => {
                TypeRef::UniRef(id)
            }
            TypeRef::Placeholder(id) => {
                if let Some(v) = id.value(db) {
                    v.as_uni_mut(db)
                } else {
                    TypeRef::Placeholder(id.as_uni_mut())
                }
            }
            _ => self,
        }
    }

    pub fn force_as_uni_mut(self, db: &Database) -> Self {
        match self {
            TypeRef::Owned(id)
            | TypeRef::Any(id)
            | TypeRef::Uni(id)
            | TypeRef::Mut(id)
            | TypeRef::Ref(id) => TypeRef::UniMut(id),
            TypeRef::Placeholder(id) => {
                if let Some(v) = id.value(db) {
                    v.force_as_uni_mut(db)
                } else {
                    TypeRef::Placeholder(id.as_uni_mut())
                }
            }
            _ => self,
        }
    }

    pub fn as_uni(self, db: &Database) -> Self {
        if self.is_value_type(db) {
            return self;
        }

        match self {
            TypeRef::Owned(id)
            | TypeRef::Any(id)
            | TypeRef::Uni(id)
            | TypeRef::Mut(id)
            | TypeRef::Ref(id) => TypeRef::Uni(id),
            TypeRef::Placeholder(id) => {
                if let Some(v) = id.value(db) {
                    v.as_uni(db)
                } else {
                    TypeRef::Placeholder(id.as_uni())
                }
            }
            _ => self,
        }
    }

    pub fn as_owned(self, db: &Database) -> Self {
        match self {
            TypeRef::Uni(id)
            | TypeRef::Any(id)
            | TypeRef::Ref(id)
            | TypeRef::Mut(id)
            | TypeRef::UniRef(id)
            | TypeRef::UniMut(id) => TypeRef::Owned(id),
            TypeRef::Placeholder(id) => {
                if let Some(v) = id.value(db) {
                    v.as_owned(db)
                } else {
                    TypeRef::Placeholder(id.as_owned())
                }
            }
            _ => self,
        }
    }

    pub fn as_enum_instance(self, db: &Database) -> Option<ClassInstance> {
        match self {
            TypeRef::Owned(TypeId::ClassInstance(ins))
            | TypeRef::Uni(TypeId::ClassInstance(ins))
            | TypeRef::Ref(TypeId::ClassInstance(ins))
            | TypeRef::Mut(TypeId::ClassInstance(ins))
                if ins.instance_of.kind(db).is_enum() =>
            {
                Some(ins)
            }
            _ => None,
        }
    }

    pub fn as_trait_instance(self, db: &Database) -> Option<TraitInstance> {
        if let Ok(TypeId::TraitInstance(ins)) = self.type_id(db) {
            Some(ins)
        } else {
            None
        }
    }

    pub fn as_class_instance(self, db: &Database) -> Option<ClassInstance> {
        if let Ok(TypeId::ClassInstance(ins)) = self.type_id(db) {
            Some(ins)
        } else {
            None
        }
    }

    pub fn as_class(self, db: &Database) -> Option<ClassId> {
        match self {
            TypeRef::Owned(TypeId::Class(id)) => Some(id),
            TypeRef::Owned(TypeId::Module(id)) => Some(id.class(db)),
            _ => None,
        }
    }

    pub fn as_type_parameter(self, db: &Database) -> Option<TypeParameterId> {
        match self {
            TypeRef::Owned(TypeId::TypeParameter(id))
            | TypeRef::Uni(TypeId::TypeParameter(id))
            | TypeRef::Ref(TypeId::TypeParameter(id))
            | TypeRef::Mut(TypeId::TypeParameter(id))
            | TypeRef::Any(TypeId::TypeParameter(id))
            | TypeRef::Owned(TypeId::RigidTypeParameter(id))
            | TypeRef::Uni(TypeId::RigidTypeParameter(id))
            | TypeRef::Ref(TypeId::RigidTypeParameter(id))
            | TypeRef::Mut(TypeId::RigidTypeParameter(id))
            | TypeRef::UniRef(TypeId::RigidTypeParameter(id))
            | TypeRef::UniMut(TypeId::RigidTypeParameter(id))
            | TypeRef::Any(TypeId::RigidTypeParameter(id)) => Some(id),
            TypeRef::Placeholder(id) => {
                id.value(db).and_then(|v| v.as_type_parameter(db))
            }
            _ => None,
        }
    }

    pub fn fields(self, db: &Database) -> Vec<FieldId> {
        match self {
            TypeRef::Owned(TypeId::ClassInstance(ins))
            | TypeRef::Uni(TypeId::ClassInstance(ins))
            | TypeRef::Mut(TypeId::ClassInstance(ins))
            | TypeRef::Ref(TypeId::ClassInstance(ins))
            | TypeRef::UniRef(TypeId::ClassInstance(ins))
            | TypeRef::UniMut(TypeId::ClassInstance(ins)) => {
                ins.instance_of().fields(db)
            }
            TypeRef::Placeholder(id) => {
                id.value(db).map_or_else(Vec::new, |v| v.fields(db))
            }
            _ => Vec::new(),
        }
    }

    pub fn as_rigid_type(self, db: &mut Database, bounds: &TypeBounds) -> Self {
        TypeResolver::new(db, &TypeArguments::new(), bounds)
            .with_rigid(true)
            .resolve(self)
    }

    pub fn as_rigid_type_parameter(self) -> TypeRef {
        match self {
            TypeRef::Owned(TypeId::TypeParameter(id)) => {
                TypeRef::Owned(TypeId::RigidTypeParameter(id))
            }
            TypeRef::Any(TypeId::TypeParameter(id)) => {
                TypeRef::Any(TypeId::RigidTypeParameter(id))
            }
            TypeRef::Ref(TypeId::TypeParameter(id)) => {
                TypeRef::Ref(TypeId::RigidTypeParameter(id))
            }
            TypeRef::Mut(TypeId::TypeParameter(id)) => {
                TypeRef::Mut(TypeId::RigidTypeParameter(id))
            }
            TypeRef::Uni(TypeId::TypeParameter(id)) => {
                TypeRef::Uni(TypeId::RigidTypeParameter(id))
            }
            TypeRef::UniRef(TypeId::TypeParameter(id)) => {
                TypeRef::UniRef(TypeId::RigidTypeParameter(id))
            }
            TypeRef::UniMut(TypeId::TypeParameter(id)) => {
                TypeRef::UniMut(TypeId::RigidTypeParameter(id))
            }
            _ => self,
        }
    }

    /// Returns `true` if `self` is a value type.
    ///
    /// Value types are types that use atomic reference counting (processes and
    /// strings), those allocated on the stack (Int, pointers, inline types,
    /// etc), or non-values (e.g. modules).
    pub fn is_value_type(self, db: &Database) -> bool {
        match self {
            TypeRef::Owned(TypeId::ClassInstance(ins))
            | TypeRef::Ref(TypeId::ClassInstance(ins))
            | TypeRef::Mut(TypeId::ClassInstance(ins))
            | TypeRef::UniRef(TypeId::ClassInstance(ins))
            | TypeRef::UniMut(TypeId::ClassInstance(ins))
            | TypeRef::Uni(TypeId::ClassInstance(ins)) => {
                ins.instance_of().is_value_type(db)
            }
            // Modules technically aren't values, but this allows certain checks
            // for value types (e.g. to see if `self` can be captured) to
            // automatically also handle modules.
            TypeRef::Owned(TypeId::Module(_))
            | TypeRef::Ref(TypeId::Module(_))
            | TypeRef::Mut(TypeId::Module(_)) => true,
            TypeRef::Owned(TypeId::Foreign(_)) => true,
            TypeRef::Pointer(_) => true,
            TypeRef::Placeholder(id) => {
                id.value(db).map_or(false, |v| v.is_value_type(db))
            }
            _ => false,
        }
    }

    /// Returns `true` if the type is allocated on the stack.
    pub fn is_stack_allocated(self, db: &Database) -> bool {
        match self {
            TypeRef::Owned(id)
            | TypeRef::Uni(id)
            | TypeRef::Ref(id)
            | TypeRef::UniRef(id)
            | TypeRef::Mut(id)
            | TypeRef::UniMut(id)
            | TypeRef::Any(id) => match id {
                TypeId::ClassInstance(ins) => {
                    ins.instance_of().is_stack_allocated(db)
                }
                TypeId::TypeParameter(tid)
                | TypeId::RigidTypeParameter(tid) => tid.is_stack_allocated(db),
                TypeId::Foreign(_) => true,
                _ => false,
            },
            TypeRef::Error | TypeRef::Pointer(_) => true,
            TypeRef::Placeholder(id) => {
                id.value(db).map_or(false, |v| v.is_stack_allocated(db))
            }
            TypeRef::Never | TypeRef::Unknown => false,
        }
    }

    pub fn is_inferred(self, db: &Database) -> bool {
        match self {
            TypeRef::Owned(id)
            | TypeRef::Uni(id)
            | TypeRef::Ref(id)
            | TypeRef::Mut(id)
            | TypeRef::UniRef(id)
            | TypeRef::UniMut(id)
            | TypeRef::Any(id) => match id {
                TypeId::ClassInstance(ins)
                    if ins.instance_of.is_generic(db) =>
                {
                    ins.type_arguments(db)
                        .unwrap()
                        .mapping
                        .values()
                        .all(|v| v.is_inferred(db))
                }
                TypeId::TraitInstance(ins)
                    if ins.instance_of.is_generic(db) =>
                {
                    ins.type_arguments(db)
                        .unwrap()
                        .mapping
                        .values()
                        .all(|v| v.is_inferred(db))
                }
                TypeId::Closure(id) => {
                    id.arguments(db)
                        .into_iter()
                        .all(|arg| arg.value_type.is_inferred(db))
                        && id.return_type(db).is_inferred(db)
                }
                _ => true,
            },
            TypeRef::Placeholder(id) => {
                id.value(db).map_or(false, |v| v.is_inferred(db))
            }
            _ => true,
        }
    }

    pub fn class_id(self, db: &Database) -> Option<ClassId> {
        match self {
            TypeRef::Owned(TypeId::ClassInstance(ins))
            | TypeRef::Uni(TypeId::ClassInstance(ins))
            | TypeRef::Ref(TypeId::ClassInstance(ins))
            | TypeRef::Mut(TypeId::ClassInstance(ins))
            | TypeRef::UniMut(TypeId::ClassInstance(ins))
            | TypeRef::UniRef(TypeId::ClassInstance(ins))
            | TypeRef::Pointer(TypeId::ClassInstance(ins)) => {
                Some(ins.instance_of)
            }
            TypeRef::Owned(TypeId::Class(id)) => Some(id),
            TypeRef::Owned(TypeId::Module(id)) => Some(id.class(db)),
            TypeRef::Placeholder(p) => p.value(db).and_then(|v| v.class_id(db)),
            _ => None,
        }
    }

    pub fn throw_kind(self, db: &Database) -> ThrowKind {
        match self {
            TypeRef::Owned(TypeId::ClassInstance(ins))
            | TypeRef::Uni(TypeId::ClassInstance(ins))
            | TypeRef::Ref(TypeId::ClassInstance(ins))
            | TypeRef::Mut(TypeId::ClassInstance(ins))
            | TypeRef::UniMut(TypeId::ClassInstance(ins))
            | TypeRef::UniRef(TypeId::ClassInstance(ins)) => {
                let opt_class = db.class_in_module(OPTION_MODULE, OPTION_CLASS);
                let res_class = db.class_in_module(RESULT_MODULE, RESULT_CLASS);
                let params = ins.instance_of.type_parameters(db);

                if ins.instance_of == res_class {
                    let args = ins.type_arguments(db).unwrap();
                    let ok = args.get(params[0]).unwrap();
                    let err = args.get(params[1]).unwrap();

                    ThrowKind::Result(ok, err)
                } else if ins.instance_of == opt_class {
                    let args = ins.type_arguments(db).unwrap();
                    let some = args.get(params[0]).unwrap();

                    ThrowKind::Option(some)
                } else {
                    ThrowKind::Unknown
                }
            }
            TypeRef::Placeholder(p) => {
                p.value(db).map_or(ThrowKind::Infer(p), |v| v.throw_kind(db))
            }
            _ => ThrowKind::Unknown,
        }
    }

    pub fn result_type(
        db: &mut Database,
        ok: TypeRef,
        error: TypeRef,
    ) -> TypeRef {
        let class = db.class_in_module(RESULT_MODULE, RESULT_CLASS);
        let params = class.type_parameters(db);
        let mut args = TypeArguments::new();

        args.assign(params[0], ok);
        args.assign(params[1], error);

        TypeRef::Owned(TypeId::ClassInstance(ClassInstance::generic(
            db, class, args,
        )))
    }

    pub fn option_type(db: &mut Database, some: TypeRef) -> TypeRef {
        let class = db.class_in_module(OPTION_MODULE, OPTION_CLASS);
        let params = class.type_parameters(db);
        let mut args = TypeArguments::new();

        args.assign(params[0], some);

        TypeRef::Owned(TypeId::ClassInstance(ClassInstance::generic(
            db, class, args,
        )))
    }

    pub fn shape(
        self,
        db: &Database,
        interned: &mut InternedTypeArguments,
        shapes: &HashMap<TypeParameterId, Shape>,
    ) -> Shape {
        match self {
            TypeRef::Owned(TypeId::ClassInstance(ins))
            | TypeRef::Uni(TypeId::ClassInstance(ins)) => {
                ins.shape(db, interned, Shape::Owned)
            }
            TypeRef::Mut(TypeId::ClassInstance(ins))
            | TypeRef::UniMut(TypeId::ClassInstance(ins)) => {
                ins.shape(db, interned, Shape::Mut)
            }
            TypeRef::Ref(TypeId::ClassInstance(ins))
            | TypeRef::UniRef(TypeId::ClassInstance(ins)) => {
                ins.shape(db, interned, Shape::Ref)
            }
            TypeRef::Any(
                TypeId::TypeParameter(id) | TypeId::RigidTypeParameter(id),
            )
            | TypeRef::Owned(
                TypeId::TypeParameter(id) | TypeId::RigidTypeParameter(id),
            )
            | TypeRef::Uni(
                TypeId::TypeParameter(id) | TypeId::RigidTypeParameter(id),
            ) => {
                // We panic if a shape is missing, as encountering a missing
                // shape is the result of a compiler bug.
                shapes.get(&id).cloned().unwrap_or_else(|| {
                    panic!(
                        "type parameter '{}' (ID: {}) must be assigned a shape",
                        id.name(db),
                        id.0
                    )
                })
            }
            // These types are the result of specialization, so we can return
            // the shape directly instead of looking at `shapes`.
            TypeRef::Owned(TypeId::AtomicTypeParameter(_))
            | TypeRef::Ref(TypeId::AtomicTypeParameter(_))
            | TypeRef::Mut(TypeId::AtomicTypeParameter(_)) => Shape::Atomic,
            TypeRef::Mut(
                TypeId::TypeParameter(id) | TypeId::RigidTypeParameter(id),
            )
            | TypeRef::UniMut(
                TypeId::TypeParameter(id) | TypeId::RigidTypeParameter(id),
            ) => match shapes.get(&id).cloned() {
                Some(Shape::Owned) | None => Shape::Mut,
                Some(shape) => shape,
            },
            TypeRef::Ref(
                TypeId::TypeParameter(id) | TypeId::RigidTypeParameter(id),
            )
            | TypeRef::UniRef(
                TypeId::TypeParameter(id) | TypeId::RigidTypeParameter(id),
            ) => match shapes.get(&id).cloned() {
                Some(Shape::Owned) | None => Shape::Ref,
                Some(shape) => shape,
            },
            TypeRef::Mut(_) | TypeRef::UniMut(_) => Shape::Mut,
            TypeRef::Ref(_) | TypeRef::UniRef(_) => Shape::Ref,
            TypeRef::Placeholder(id) => id
                .value(db)
                .map_or(Shape::Owned, |v| v.shape(db, interned, shapes)),
            TypeRef::Owned(TypeId::Foreign(ForeignType::Int(size, sign)))
            | TypeRef::Uni(TypeId::Foreign(ForeignType::Int(size, sign))) => {
                Shape::Int(size, sign)
            }
            TypeRef::Owned(TypeId::Foreign(ForeignType::Float(size)))
            | TypeRef::Uni(TypeId::Foreign(ForeignType::Float(size))) => {
                Shape::Float(size)
            }
            TypeRef::Pointer(_) => Shape::Pointer,
            _ => Shape::Owned,
        }
    }

    pub fn is_signed_int(self, db: &Database) -> bool {
        let Ok(id) = self.type_id(db) else { return false };

        match id {
            TypeId::Foreign(ForeignType::Int(_, Sign::Signed)) => true,
            TypeId::ClassInstance(ins) => ins.instance_of().0 == INT_ID,
            _ => false,
        }
    }

    fn is_instance_of(self, db: &Database, id: ClassId) -> bool {
        self.class_id(db) == Some(id)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum ForeignType {
    Int(u32, Sign),
    Float(u32),
}

/// An ID pointing to a type.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum TypeId {
    Class(ClassId),
    Trait(TraitId),
    Module(ModuleId),
    ClassInstance(ClassInstance),
    TraitInstance(TraitInstance),
    TypeParameter(TypeParameterId),
    RigidTypeParameter(TypeParameterId),

    /// A type parameter that uses atomic reference counting for aliases.
    ///
    /// This constructor isn't produced by users through the type system,
    /// instead it's produced when specializing type parameters that are
    /// assigned atomic types, such as processes.
    AtomicTypeParameter(TypeParameterId),
    Closure(ClosureId),
    Foreign(ForeignType),
}

impl TypeId {
    pub fn named_type(self, db: &mut Database, name: &str) -> Option<Symbol> {
        match self {
            TypeId::Module(id) => id.use_symbol(db, name),
            TypeId::Trait(id) => id.named_type(db, name),
            TypeId::Class(id) => id.named_type(db, name),
            TypeId::ClassInstance(id) => id.named_type(db, name),
            TypeId::TraitInstance(id) => id.named_type(db, name),
            _ => None,
        }
    }

    pub fn lookup_method(
        self,
        db: &Database,
        name: &str,
        module: ModuleId,
        allow_type_private: bool,
    ) -> MethodLookup {
        if let Some(id) = self.method(db, name) {
            let kind = id.kind(db);
            let is_ins = !matches!(
                self,
                TypeId::Class(_) | TypeId::Trait(_) | TypeId::Module(_)
            );

            if is_ins && kind.is_static() {
                MethodLookup::StaticOnInstance
            } else if !is_ins && !kind.is_static() {
                MethodLookup::InstanceOnStatic
            } else if self.can_call(db, id, module, allow_type_private) {
                MethodLookup::Ok(id)
            } else {
                MethodLookup::Private
            }
        } else if let TypeId::Module(id) = self {
            id.extern_method(db, name)
                .map(MethodLookup::Ok)
                .unwrap_or(MethodLookup::None)
        } else {
            MethodLookup::None
        }
    }

    pub fn method(self, db: &Database, name: &str) -> Option<MethodId> {
        match self {
            TypeId::Class(id) => id.method(db, name),
            TypeId::Trait(id) => id.method(db, name),
            TypeId::Module(id) => id.method(db, name),
            TypeId::ClassInstance(id) => id.method(db, name),
            TypeId::TraitInstance(id) => id.method(db, name),
            TypeId::TypeParameter(id) | TypeId::RigidTypeParameter(id) => {
                id.method(db, name)
            }
            _ => None,
        }
    }

    pub fn use_dynamic_dispatch(self) -> bool {
        matches!(
            self,
            TypeId::TraitInstance(_)
                | TypeId::TypeParameter(_)
                | TypeId::RigidTypeParameter(_)
        )
    }

    pub fn has_destructor(self, db: &Database) -> bool {
        if let TypeId::ClassInstance(id) = self {
            id.instance_of().has_destructor(db)
        } else {
            false
        }
    }

    pub fn as_type_for_pointer(self) -> TypeRef {
        if let TypeId::TypeParameter(_) | TypeId::RigidTypeParameter(_) = self {
            TypeRef::Any(self)
        } else {
            TypeRef::Owned(self)
        }
    }

    fn can_call(
        self,
        db: &Database,
        method: MethodId,
        module: ModuleId,
        allow_type_private: bool,
    ) -> bool {
        let m = method.get(db);

        if m.kind == MethodKind::Destructor {
            return false;
        }

        match m.visibility {
            Visibility::Public => true,
            Visibility::Private => m.module.has_same_root_namespace(db, module),
            Visibility::TypePrivate => allow_type_private,
        }
    }
}

/// A database of all Inko types.
pub struct Database {
    modules: Vec<Module>,
    module_mapping: HashMap<String, ModuleId>,
    traits: Vec<Trait>,
    classes: Vec<Class>,
    type_parameters: Vec<TypeParameter>,
    type_arguments: Vec<TypeArguments>,
    methods: Vec<Method>,
    fields: Vec<Field>,
    closures: Vec<Closure>,
    variables: Vec<Variable>,
    constants: Vec<Constant>,
    intrinsics: HashMap<String, Intrinsic>,
    type_placeholders: Vec<TypePlaceholder>,
    constructors: Vec<Constructor>,

    /// The module that acts as the entry point of the program.
    ///
    /// For executables this will be set based on the file that is built/run.
    /// When just type-checking a project, this may be left as a None.
    main_module: Option<ModuleName>,
    main_method: Option<MethodId>,
    main_class: Option<ClassId>,
}

impl Database {
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
            module_mapping: HashMap::new(),
            traits: Vec::new(),
            classes: vec![
                Class::atomic(STRING_NAME.to_string()),
                Class::regular(BYTE_ARRAY_NAME.to_string()),
                Class::value_type(INT_NAME.to_string()),
                Class::value_type(FLOAT_NAME.to_string()),
                Class::value_type(BOOL_NAME.to_string()),
                Class::value_type(NIL_NAME.to_string()),
                Class::tuple(TUPLE1_NAME.to_string()),
                Class::tuple(TUPLE2_NAME.to_string()),
                Class::tuple(TUPLE3_NAME.to_string()),
                Class::tuple(TUPLE4_NAME.to_string()),
                Class::tuple(TUPLE5_NAME.to_string()),
                Class::tuple(TUPLE6_NAME.to_string()),
                Class::tuple(TUPLE7_NAME.to_string()),
                Class::tuple(TUPLE8_NAME.to_string()),
                Class::regular(ARRAY_NAME.to_string()),
                Class::new(
                    CHECKED_INT_RESULT_NAME.to_string(),
                    ClassKind::Extern,
                    Visibility::Private,
                    ModuleId(DEFAULT_BUILTIN_MODULE_ID),
                    Location::default(),
                ),
            ],
            type_parameters: Vec::new(),
            type_arguments: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            closures: Vec::new(),
            variables: Vec::new(),
            constants: Vec::new(),
            intrinsics: Intrinsic::mapping(),
            type_placeholders: Vec::new(),
            constructors: Vec::new(),
            main_module: None,
            main_method: None,
            main_class: None,
        }
    }

    pub fn compact(&mut self) {
        // After specialization, the type arguments are no longer in use.
        // Removing them here frees the memory, and ensures we don't continue to
        // use them by mistake.
        self.type_arguments.clear();
        self.type_arguments.shrink_to_fit();
    }

    pub fn builtin_class(&self, name: &str) -> Option<ClassId> {
        match name {
            INT_NAME => Some(ClassId::int()),
            FLOAT_NAME => Some(ClassId::float()),
            STRING_NAME => Some(ClassId::string()),
            ARRAY_NAME => Some(ClassId::array()),
            BOOL_NAME => Some(ClassId::boolean()),
            NIL_NAME => Some(ClassId::nil()),
            BYTE_ARRAY_NAME => Some(ClassId::byte_array()),
            TUPLE1_NAME => Some(ClassId::tuple1()),
            TUPLE2_NAME => Some(ClassId::tuple2()),
            TUPLE3_NAME => Some(ClassId::tuple3()),
            TUPLE4_NAME => Some(ClassId::tuple4()),
            TUPLE5_NAME => Some(ClassId::tuple5()),
            TUPLE6_NAME => Some(ClassId::tuple6()),
            TUPLE7_NAME => Some(ClassId::tuple7()),
            TUPLE8_NAME => Some(ClassId::tuple8()),
            CHECKED_INT_RESULT_NAME => Some(ClassId::checked_int_result()),
            _ => None,
        }
    }

    pub fn intrinsic(&self, name: &str) -> Option<Intrinsic> {
        self.intrinsics.get(name).cloned()
    }

    pub fn module(&self, name: &str) -> ModuleId {
        if let Some(id) = self.optional_module(name) {
            return id;
        }

        panic!("The module '{}' isn't registered in the type database", name);
    }

    pub fn optional_module(&self, name: &str) -> Option<ModuleId> {
        self.module_mapping.get(name).cloned()
    }

    pub fn class_in_module(&self, module: &str, name: &str) -> ClassId {
        if let Some(Symbol::Class(id)) = self.module(module).symbol(self, name)
        {
            id
        } else {
            panic!("The class {}.{} isn't defined", module, name)
        }
    }

    pub fn trait_in_module(&self, module: &str, name: &str) -> TraitId {
        if let Some(Symbol::Trait(id)) = self.module(module).symbol(self, name)
        {
            id
        } else {
            panic!("The trait {}.{} isn't defined", module, name)
        }
    }

    pub fn drop_trait(&self) -> TraitId {
        self.trait_in_module(DROP_MODULE, DROP_TRAIT)
    }

    pub fn number_of_traits(&self) -> usize {
        self.traits.len()
    }

    pub fn number_of_modules(&self) -> usize {
        self.modules.len()
    }

    pub fn number_of_classes(&self) -> usize {
        self.classes.len()
    }

    pub fn number_of_methods(&self) -> usize {
        self.methods.len()
    }

    pub fn set_main_module(&mut self, name: ModuleName) {
        self.main_module = Some(name);
    }

    pub fn main_module(&self) -> Option<&ModuleName> {
        self.main_module.as_ref()
    }

    pub fn set_main_method(&mut self, id: MethodId) {
        self.main_method = Some(id);
    }

    pub fn main_method(&self) -> Option<MethodId> {
        self.main_method
    }

    pub fn set_main_class(&mut self, id: ClassId) {
        self.main_class = Some(id);
    }

    pub fn main_class(&self) -> Option<ClassId> {
        self.main_class
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::{
        any, closure, generic_instance_id, generic_trait_instance, immutable,
        immutable_uni, instance, mutable, mutable_uni, new_async_class,
        new_class, new_enum_class, new_extern_class, new_module, new_parameter,
        new_trait, owned, parameter, placeholder, pointer, rigid,
        trait_instance, uni,
    };
    use std::mem::size_of;

    fn assert_sync<T: Sync>() {}

    #[test]
    fn test_type_sizes() {
        assert_eq!(size_of::<TypeId>(), 16);
        assert_eq!(size_of::<TypeRef>(), 24);
        assert_eq!(size_of::<ForeignType>(), 8);
    }

    #[test]
    fn test_sync() {
        assert_sync::<TypePlaceholder>();
        assert_sync::<Database>();
    }

    #[test]
    fn test_type_parameter_alloc() {
        let mut db = Database::new();
        let id = TypeParameter::alloc(&mut db, "A".to_string());

        assert_eq!(id.0, 0);
        assert_eq!(&db.type_parameters[0].name, &"A".to_string());
    }

    #[test]
    fn test_type_parameter_clone_for_bound() {
        let mut db = Database::new();
        let param1 = TypeParameter::alloc(&mut db, "A".to_string());

        param1.set_mutable(&mut db);
        param1.set_stack_allocated(&mut db);

        let param2 = param1.clone_for_bound(&mut db);

        assert_eq!(param1.is_mutable(&db), param2.is_mutable(&db));
        assert_eq!(
            param1.is_stack_allocated(&db),
            param2.is_stack_allocated(&db)
        );
    }

    #[test]
    fn test_type_parameter_new() {
        let param = TypeParameter::new("A".to_string());

        assert_eq!(&param.name, &"A");
        assert!(param.requirements.is_empty());
    }

    #[test]
    fn test_type_parameter_id_name() {
        let mut db = Database::new();
        let id = TypeParameter::alloc(&mut db, "A".to_string());

        assert_eq!(id.name(&db), &"A");
    }

    #[test]
    fn test_type_parameter_id_add_requirements() {
        let mut db = Database::new();
        let id = TypeParameter::alloc(&mut db, "A".to_string());
        let trait_id = Trait::alloc(
            &mut db,
            "ToString".to_string(),
            Visibility::Private,
            ModuleId(0),
            Location::default(),
        );
        let requirement = TraitInstance::new(trait_id);

        id.add_requirements(&mut db, vec![requirement]);

        assert_eq!(id.requirements(&db), vec![requirement]);
    }

    #[test]
    fn test_type_arguments_assign() {
        let mut targs = TypeArguments::new();
        let mut db = Database::new();
        let param1 = TypeParameter::alloc(&mut db, "A".to_string());
        let param2 = TypeParameter::alloc(&mut db, "B".to_string());

        targs.assign(param1, TypeRef::Never);

        assert_eq!(targs.get(param1), Some(TypeRef::Never));
        assert_eq!(targs.get(param2), None);
        assert_eq!(targs.mapping.len(), 1);
    }

    #[test]
    fn test_type_arguments_get_recursive() {
        let mut db = Database::new();
        let mut targs = TypeArguments::new();
        let param1 = new_parameter(&mut db, "A");
        let param2 = new_parameter(&mut db, "B");
        let param3 = new_parameter(&mut db, "C");
        let param4 = new_parameter(&mut db, "D");
        let param5 = new_parameter(&mut db, "E");
        let param6 = new_parameter(&mut db, "F");
        let param7 = new_parameter(&mut db, "G");
        let param8 = new_parameter(&mut db, "H");

        targs.assign(param1, any(parameter(param2)));
        targs.assign(param2, owned(rigid(param3)));
        targs.assign(param3, TypeRef::int());
        targs.assign(param5, TypeRef::float());
        targs.assign(param6, owned(rigid(param6)));
        targs.assign(param7, owned(rigid(param8)));

        assert_eq!(targs.get_recursive(&db, param1), Some(TypeRef::int()));
        assert_eq!(targs.get_recursive(&db, param2), Some(TypeRef::int()));
        assert_eq!(targs.get_recursive(&db, param3), Some(TypeRef::int()));
        assert_eq!(targs.get_recursive(&db, param4), None);
        assert_eq!(targs.get_recursive(&db, param5), Some(TypeRef::float()));
        assert_eq!(
            targs.get_recursive(&db, param6),
            Some(owned(rigid(param6)))
        );

        assert_eq!(
            targs.get_recursive(&db, param7),
            Some(owned(rigid(param8)))
        );
    }

    #[test]
    fn test_trait_alloc() {
        let mut db = Database::new();
        let id = Trait::alloc(
            &mut db,
            "A".to_string(),
            Visibility::Private,
            ModuleId(0),
            Location::default(),
        );

        assert_eq!(id.0, 0);
        assert_eq!(&db.traits[0].name, &"A".to_string());
    }

    #[test]
    fn test_trait_new() {
        let trait_type = Trait::new(
            "A".to_string(),
            Visibility::Private,
            ModuleId(0),
            Location::default(),
        );

        assert_eq!(&trait_type.name, &"A");
    }

    #[test]
    fn test_trait_id_new_type_parameter() {
        let mut db = Database::new();
        let id = Trait::alloc(
            &mut db,
            "A".to_string(),
            Visibility::Private,
            ModuleId(0),
            Location::default(),
        );
        let param = id.new_type_parameter(&mut db, "A".to_string());

        assert_eq!(id.type_parameters(&db), vec![param]);
    }

    #[test]
    fn test_trait_instance_new() {
        let mut db = Database::new();
        let id = Trait::alloc(
            &mut db,
            "A".to_string(),
            Visibility::Private,
            ModuleId(0),
            Location::default(),
        );
        let ins = TraitInstance::new(id);
        let index = db.traits.len() as u32 - 1;

        assert_eq!(ins.instance_of.0, index);
        assert_eq!(ins.type_arguments, 0);
    }

    #[test]
    fn test_trait_instance_generic() {
        let mut db = Database::new();
        let id = Trait::alloc(
            &mut db,
            "A".to_string(),
            Visibility::Private,
            ModuleId(0),
            Location::default(),
        );
        let ins1 = TraitInstance::generic(&mut db, id, TypeArguments::new());
        let ins2 = TraitInstance::generic(&mut db, id, TypeArguments::new());
        let index = db.traits.len() as u32 - 1;

        assert_eq!(ins1.instance_of.0, index);
        assert_eq!(ins1.type_arguments, 0);

        assert_eq!(ins2.instance_of.0, index);
        assert_eq!(ins2.type_arguments, 1);
    }

    #[test]
    fn test_class_alloc() {
        let mut db = Database::new();
        let id = Class::alloc(
            &mut db,
            "A".to_string(),
            ClassKind::Regular,
            Visibility::Private,
            ModuleId(0),
            Location::default(),
        );

        assert_eq!(id.0, FIRST_USER_CLASS_ID);
        assert_eq!(
            &db.classes[FIRST_USER_CLASS_ID as usize].name,
            &"A".to_string()
        );
        assert_eq!(
            db.classes[FIRST_USER_CLASS_ID as usize].kind,
            ClassKind::Regular
        );
    }

    #[test]
    fn test_class_clone_for_specialization() {
        let mut db = Database::new();
        let class1 = new_class(&mut db, "A");

        class1.set_stack_allocated(&mut db);

        let class2 = class1.clone_for_specialization(&mut db);

        assert!(class2.is_stack_allocated(&db));
    }

    #[test]
    fn test_class_new() {
        let class = Class::new(
            "A".to_string(),
            ClassKind::Async,
            Visibility::Private,
            ModuleId(0),
            Location::default(),
        );

        assert_eq!(&class.name, &"A");
        assert_eq!(class.kind, ClassKind::Async);
    }

    #[test]
    fn test_class_id_name() {
        let mut db = Database::new();
        let id = Class::alloc(
            &mut db,
            "A".to_string(),
            ClassKind::Regular,
            Visibility::Private,
            ModuleId(0),
            Location::default(),
        );

        assert_eq!(id.name(&db), &"A");
    }

    #[test]
    fn test_class_id_is_async() {
        let mut db = Database::new();
        let regular_class = Class::alloc(
            &mut db,
            "A".to_string(),
            ClassKind::Regular,
            Visibility::Private,
            ModuleId(0),
            Location::default(),
        );
        let async_class = Class::alloc(
            &mut db,
            "A".to_string(),
            ClassKind::Async,
            Visibility::Private,
            ModuleId(0),
            Location::default(),
        );

        assert!(!regular_class.kind(&db).is_async());
        assert!(async_class.kind(&db).is_async());
    }

    #[test]
    fn test_class_id_new_type_parameter() {
        let mut db = Database::new();
        let id = Class::alloc(
            &mut db,
            "A".to_string(),
            ClassKind::Regular,
            Visibility::Private,
            ModuleId(0),
            Location::default(),
        );
        let param = id.new_type_parameter(&mut db, "A".to_string());

        assert_eq!(id.type_parameters(&db), vec![param]);
    }

    #[test]
    fn test_class_instance_new() {
        let mut db = Database::new();
        let id = Class::alloc(
            &mut db,
            "A".to_string(),
            ClassKind::Regular,
            Visibility::Private,
            ModuleId(0),
            Location::default(),
        );
        let ins = ClassInstance::new(id);

        assert_eq!(ins.instance_of.0, FIRST_USER_CLASS_ID);
        assert_eq!(ins.type_arguments, 0);
    }

    #[test]
    fn test_class_instance_generic() {
        let mut db = Database::new();
        let id = Class::alloc(
            &mut db,
            "A".to_string(),
            ClassKind::Regular,
            Visibility::Private,
            ModuleId(0),
            Location::default(),
        );
        let ins1 = ClassInstance::generic(&mut db, id, TypeArguments::new());
        let ins2 = ClassInstance::generic(&mut db, id, TypeArguments::new());

        assert_eq!(ins1.instance_of.0, FIRST_USER_CLASS_ID);
        assert_eq!(ins1.type_arguments, 0);

        assert_eq!(ins2.instance_of.0, FIRST_USER_CLASS_ID);
        assert_eq!(ins2.type_arguments, 1);
    }

    #[test]
    fn test_method_alloc() {
        let mut db = Database::new();
        let id = Method::alloc(
            &mut db,
            ModuleId(0),
            Location::default(),
            "foo".to_string(),
            Visibility::Private,
            MethodKind::Moving,
        );

        assert_eq!(id.0, 0);
        assert_eq!(&db.methods[0].name, &"foo".to_string());
        assert_eq!(db.methods[0].kind, MethodKind::Moving);
    }

    #[test]
    fn test_method_alloc_extern() {
        let mut db = Database::new();
        let id = Method::alloc(
            &mut db,
            ModuleId(0),
            Location::default(),
            "foo".to_string(),
            Visibility::Private,
            MethodKind::Extern,
        );

        assert_eq!(id.inline(&db), Inline::Never);
    }

    #[test]
    fn test_method_set_never_return_type() {
        let mut db = Database::new();
        let id = Method::alloc(
            &mut db,
            ModuleId(0),
            Location::default(),
            "foo".to_string(),
            Visibility::Private,
            MethodKind::Instance,
        );

        id.set_return_type(&mut db, TypeRef::Never);
        assert_eq!(id.inline(&db), Inline::Never);
    }

    #[test]
    fn test_method_id_named_type() {
        let mut db = Database::new();
        let method = Method::alloc(
            &mut db,
            ModuleId(0),
            Location::default(),
            "foo".to_string(),
            Visibility::Private,
            MethodKind::Instance,
        );
        let param = method.new_type_parameter(&mut db, "A".to_string());

        assert_eq!(
            method.named_type(&db, "A"),
            Some(Symbol::TypeParameter(param))
        );
    }

    #[test]
    fn test_method_id_file() {
        let mut db = Database::new();
        let mod1 = new_module(&mut db, "A");
        let mod2 = new_module(&mut db, "B");
        let to_foo = Trait::alloc(
            &mut db,
            "ToFoo".to_string(),
            Visibility::Public,
            mod2,
            Location::default(),
        );

        mod2.get_mut(&mut db).file = PathBuf::from("bar.inko");

        let m1 = Method::alloc(
            &mut db,
            mod1,
            Location::default(),
            "a".to_string(),
            Visibility::Private,
            MethodKind::Instance,
        );

        let m2 = Method::alloc(
            &mut db,
            mod1,
            Location::default(),
            "a".to_string(),
            Visibility::Private,
            MethodKind::Instance,
        );

        m2.set_source(
            &mut db,
            MethodSource::Inherited(trait_instance(to_foo), m1),
        );

        assert_eq!(m1.source_file(&db).to_str(), Some("foo.inko"));
        assert_eq!(m2.source_file(&db).to_str(), Some("bar.inko"));
    }

    #[test]
    fn test_module_alloc() {
        let mut db = Database::new();
        let name = ModuleName::new("foo");
        let id = Module::alloc(&mut db, name.clone(), "foo.inko".into());

        assert_eq!(id.0, 0);
        assert_eq!(&db.modules[0].name, &name);
        assert_eq!(&db.modules[0].file, &PathBuf::from("foo.inko"));
    }

    #[test]
    fn test_module_id_file() {
        let mut db = Database::new();
        let id = Module::alloc(
            &mut db,
            ModuleName::new("foo"),
            PathBuf::from("test.inko"),
        );

        assert_eq!(id.file(&db), PathBuf::from("test.inko"));
    }

    #[test]
    fn test_module_id_symbol() {
        let mut db = Database::new();
        let id = Module::alloc(
            &mut db,
            ModuleName::new("foo"),
            PathBuf::from("test.inko"),
        );

        id.new_symbol(&mut db, "A".to_string(), Symbol::Module(id));

        assert_eq!(id.symbol(&db, "A"), Some(Symbol::Module(id)));
        assert!(!id.get(&db).symbols["A"].used);
    }

    #[test]
    fn test_module_id_use_symbol() {
        let mut db = Database::new();
        let id = Module::alloc(
            &mut db,
            ModuleName::new("foo"),
            PathBuf::from("test.inko"),
        );

        id.new_symbol(&mut db, "A".to_string(), Symbol::Module(id));

        assert_eq!(id.use_symbol(&mut db, "A"), Some(Symbol::Module(id)));
        assert!(id.get(&db).symbols["A"].used);
    }

    #[test]
    fn test_module_id_import_symbol() {
        let mut db = Database::new();
        let foo = new_module(&mut db, "foo");
        let bar = new_module(&mut db, "bar");
        let fizz = new_module(&mut db, "fizz");
        let class = new_class(&mut db, "A");
        let trait_ = Trait::alloc(
            &mut db,
            "B".to_string(),
            Visibility::Public,
            foo,
            Location::default(),
        );

        let constant = Constant::alloc(
            &mut db,
            foo,
            Location::default(),
            "C".to_string(),
            Visibility::Public,
            TypeRef::Unknown,
        );

        let method = Method::alloc(
            &mut db,
            foo,
            Location::default(),
            "D".to_string(),
            Visibility::Public,
            MethodKind::Extern,
        );

        let type_param = TypeParameter::alloc(&mut db, "E".to_string());

        class.set_module(&mut db, foo);
        foo.new_symbol(&mut db, "A".to_string(), Symbol::Class(class));
        foo.new_symbol(&mut db, "B".to_string(), Symbol::Trait(trait_));
        foo.new_symbol(&mut db, "C".to_string(), Symbol::Constant(constant));
        foo.new_symbol(&mut db, "D".to_string(), Symbol::Method(method));
        foo.new_symbol(
            &mut db,
            "E".to_string(),
            Symbol::TypeParameter(type_param),
        );

        foo.new_symbol(&mut db, "fizz".to_string(), Symbol::Module(fizz));

        bar.new_symbol(&mut db, "A".to_string(), Symbol::Class(class));
        bar.new_symbol(&mut db, "B".to_string(), Symbol::Trait(trait_));
        bar.new_symbol(&mut db, "C".to_string(), Symbol::Constant(constant));
        bar.new_symbol(&mut db, "D".to_string(), Symbol::Method(method));
        bar.new_symbol(
            &mut db,
            "E".to_string(),
            Symbol::TypeParameter(type_param),
        );

        assert_eq!(foo.import_symbol(&mut db, "unknown"), None);
        assert_eq!(foo.import_symbol(&mut db, "A"), Some(Symbol::Class(class)));
        assert_eq!(bar.import_symbol(&mut db, "A"), None);
        assert_eq!(
            foo.import_symbol(&mut db, "B"),
            Some(Symbol::Trait(trait_))
        );
        assert_eq!(bar.import_symbol(&mut db, "B"), None);
        assert_eq!(
            foo.import_symbol(&mut db, "C"),
            Some(Symbol::Constant(constant))
        );
        assert_eq!(bar.import_symbol(&mut db, "C"), None);
        assert_eq!(
            foo.import_symbol(&mut db, "D"),
            Some(Symbol::Method(method))
        );
        assert_eq!(bar.import_symbol(&mut db, "D"), None);
        assert_eq!(foo.import_symbol(&mut db, "E"), None);
        assert_eq!(bar.import_symbol(&mut db, "E"), None);
        assert_eq!(foo.import_symbol(&mut db, "fizz"), None);
    }

    #[test]
    fn test_module_id_symbols() {
        let mut db = Database::new();
        let id = Module::alloc(
            &mut db,
            ModuleName::new("foo"),
            PathBuf::from("test.inko"),
        );

        id.new_symbol(&mut db, "A".to_string(), Symbol::Module(id));

        assert_eq!(
            id.symbols(&db),
            vec![("A".to_string(), Symbol::Module(id))]
        );
    }

    #[test]
    fn test_module_id_symbol_exists() {
        let mut db = Database::new();
        let id = Module::alloc(
            &mut db,
            ModuleName::new("foo"),
            PathBuf::from("test.inko"),
        );

        id.new_symbol(&mut db, "A".to_string(), Symbol::Module(id));

        assert!(id.symbol_exists(&db, "A"));
        assert!(!id.symbol_exists(&db, "B"));
    }

    #[test]
    fn test_function_closure() {
        let mut db = Database::new();
        let id = Closure::alloc(&mut db, false);

        assert_eq!(id.0, 0);
    }

    #[test]
    fn test_type_id_named_type_with_class() {
        let mut db = Database::new();
        let array = Class::alloc(
            &mut db,
            "Array".to_string(),
            ClassKind::Regular,
            Visibility::Private,
            ModuleId(0),
            Location::default(),
        );
        let param = array.new_type_parameter(&mut db, "T".to_string());

        assert_eq!(
            TypeId::Class(array).named_type(&mut db, "T"),
            Some(Symbol::TypeParameter(param))
        );
    }

    #[test]
    fn test_type_id_named_type_with_trait() {
        let mut db = Database::new();
        let to_array = Trait::alloc(
            &mut db,
            "ToArray".to_string(),
            Visibility::Private,
            ModuleId(0),
            Location::default(),
        );
        let param = to_array.new_type_parameter(&mut db, "T".to_string());

        assert_eq!(
            TypeId::Trait(to_array).named_type(&mut db, "T"),
            Some(Symbol::TypeParameter(param))
        );
    }

    #[test]
    fn test_type_id_named_type_with_module() {
        let mut db = Database::new();
        let string = Class::alloc(
            &mut db,
            "String".to_string(),
            ClassKind::Regular,
            Visibility::Private,
            ModuleId(0),
            Location::default(),
        );
        let module =
            Module::alloc(&mut db, ModuleName::new("foo"), "foo.inko".into());

        let symbol = Symbol::Class(string);
        let type_id = TypeId::Module(module);

        module.new_symbol(&mut db, "String".to_string(), symbol);

        assert_eq!(type_id.named_type(&mut db, "String"), Some(symbol));
        assert!(type_id.named_type(&mut db, "Foo").is_none());
    }

    #[test]
    fn test_type_id_named_type_with_class_instance() {
        let mut db = Database::new();
        let array = Class::alloc(
            &mut db,
            "Array".to_string(),
            ClassKind::Regular,
            Visibility::Private,
            ModuleId(0),
            Location::default(),
        );
        let param = array.new_type_parameter(&mut db, "T".to_string());
        let ins = TypeId::ClassInstance(ClassInstance::generic(
            &mut db,
            array,
            TypeArguments::new(),
        ));

        assert_eq!(
            ins.named_type(&mut db, "T"),
            Some(Symbol::TypeParameter(param))
        );
        assert!(ins.named_type(&mut db, "E").is_none());
    }

    #[test]
    fn test_type_id_named_type_with_trait_instance() {
        let mut db = Database::new();
        let to_array = Trait::alloc(
            &mut db,
            "ToArray".to_string(),
            Visibility::Private,
            ModuleId(0),
            Location::default(),
        );
        let param = to_array.new_type_parameter(&mut db, "T".to_string());
        let ins = TypeId::TraitInstance(TraitInstance::generic(
            &mut db,
            to_array,
            TypeArguments::new(),
        ));

        assert_eq!(
            ins.named_type(&mut db, "T"),
            Some(Symbol::TypeParameter(param))
        );
        assert!(ins.named_type(&mut db, "E").is_none());
    }

    #[test]
    fn test_type_id_named_type_with_type_parameter() {
        let mut db = Database::new();
        let param = TypeId::TypeParameter(TypeParameter::alloc(
            &mut db,
            "T".to_string(),
        ));

        assert!(param.named_type(&mut db, "T").is_none());
    }

    #[test]
    fn test_type_id_named_type_with_function() {
        let mut db = Database::new();
        let block = TypeId::Closure(Closure::alloc(&mut db, false));

        assert!(block.named_type(&mut db, "T").is_none());
    }

    #[test]
    fn test_database_new() {
        let db = Database::new();

        assert_eq!(&db.classes[INT_ID as usize].name, INT_NAME);
        assert_eq!(&db.classes[FLOAT_ID as usize].name, FLOAT_NAME);
        assert_eq!(&db.classes[STRING_ID as usize].name, STRING_NAME);
        assert_eq!(&db.classes[ARRAY_ID as usize].name, ARRAY_NAME);
        assert_eq!(&db.classes[BOOL_ID as usize].name, BOOL_NAME);
        assert_eq!(&db.classes[NIL_ID as usize].name, NIL_NAME);
        assert_eq!(&db.classes[BYTE_ARRAY_ID as usize].name, BYTE_ARRAY_NAME);
    }

    #[test]
    fn test_database_module() {
        let mut db = Database::new();
        let name = ModuleName::new("foo");
        let id = Module::alloc(&mut db, name, "foo.inko".into());

        assert_eq!(db.module("foo"), id);
    }

    #[test]
    #[should_panic]
    fn test_database_invalid_module() {
        let db = Database::new();

        db.module("foo");
    }

    #[test]
    fn test_class_id_is_builtin() {
        assert!(ClassId::int().is_builtin());
        assert!(!ClassId::tuple8().is_builtin());
        assert!(!ClassId(42).is_builtin());
    }

    #[test]
    fn test_type_placeholder_id_assign() {
        let mut db = Database::new();
        let param = TypeParameter::alloc(&mut db, "T".to_string());
        let p1 = TypePlaceholder::alloc(&mut db, Some(param));
        let p2 = TypePlaceholder::alloc(&mut db, Some(param));

        p1.assign(&mut db, TypeRef::int());
        p2.assign(&mut db, TypeRef::Placeholder(p2));

        assert_eq!(p1.value(&db), Some(TypeRef::int()));
        assert!(p2.value(&db).is_none());
    }

    #[test]
    fn test_type_placeholder_id_assign_with_ownership() {
        let mut db = Database::new();
        let mut var = TypePlaceholder::alloc(&mut db, None);
        let thing = new_class(&mut db, "Thing");

        var.ownership = Ownership::Owned;
        var.assign(&mut db, immutable(instance(thing)));
        assert_eq!(var.value(&db), Some(owned(instance(thing))));

        var.ownership = Ownership::Ref;
        var.assign(&mut db, owned(instance(thing)));
        assert_eq!(var.value(&db), Some(immutable(instance(thing))));

        var.ownership = Ownership::Mut;
        var.assign(&mut db, owned(instance(thing)));
        assert_eq!(var.value(&db), Some(mutable(instance(thing))));

        var.ownership = Ownership::Uni;
        var.assign(&mut db, owned(instance(thing)));
        assert_eq!(var.value(&db), Some(uni(instance(thing))));

        var.ownership = Ownership::UniRef;
        var.assign(&mut db, owned(instance(thing)));
        assert_eq!(var.value(&db), Some(immutable_uni(instance(thing))));

        var.ownership = Ownership::UniMut;
        var.assign(&mut db, owned(instance(thing)));
        assert_eq!(var.value(&db), Some(mutable_uni(instance(thing))));
    }

    #[test]
    fn test_type_placeholder_id_resolve() {
        let mut db = Database::new();
        let var1 = TypePlaceholder::alloc(&mut db, None);
        let var2 = TypePlaceholder::alloc(&mut db, None);
        let var3 = TypePlaceholder::alloc(&mut db, None);

        var1.assign(&mut db, TypeRef::int());
        var2.assign(&mut db, TypeRef::Placeholder(var1));
        var3.assign(&mut db, TypeRef::Placeholder(var2));

        assert_eq!(var1.value(&db), Some(TypeRef::int()));
        assert_eq!(var2.value(&db), Some(TypeRef::int()));
        assert_eq!(var3.value(&db), Some(TypeRef::int()));
    }

    #[test]
    fn test_type_ref_allow_as_ref() {
        let mut db = Database::new();
        let int = ClassId::int();
        let var = TypePlaceholder::alloc(&mut db, None);
        let param = new_parameter(&mut db, "A");

        var.assign(&mut db, owned(instance(int)));

        assert!(owned(instance(int)).allow_as_ref(&db));
        assert!(mutable(instance(int)).allow_as_ref(&db));
        assert!(immutable(instance(int)).allow_as_ref(&db));
        assert!(placeholder(var).allow_as_ref(&db));
        assert!(owned(rigid(param)).allow_as_ref(&db));
        assert!(uni(instance(int)).allow_as_ref(&db));
    }

    #[test]
    fn test_type_ref_as_ref() {
        let mut db = Database::new();
        let int = ClassId::int();
        let ext = new_extern_class(&mut db, "Extern");
        let p1 = new_parameter(&mut db, "A");
        let p2 = new_parameter(&mut db, "A");

        p2.set_stack_allocated(&mut db);

        assert_eq!(owned(instance(int)).as_ref(&db), owned(instance(int)));
        assert_eq!(
            uni(instance(int)).as_ref(&db),
            TypeRef::UniRef(instance(int))
        );
        assert_eq!(owned(rigid(p1)).as_ref(&db), immutable(rigid(p1)));
        assert_eq!(owned(instance(ext)).as_ref(&db), pointer(instance(ext)));

        assert_eq!(owned(parameter(p2)).as_ref(&db), owned(parameter(p2)));
        assert_eq!(immutable(parameter(p2)).as_ref(&db), owned(parameter(p2)));
        assert_eq!(mutable(parameter(p2)).as_ref(&db), owned(parameter(p2)));
        assert_eq!(owned(rigid(p2)).as_ref(&db), owned(rigid(p2)));
    }

    #[test]
    fn test_type_ref_as_mut() {
        let mut db = Database::new();
        let int = ClassId::int();
        let ext = new_extern_class(&mut db, "Extern");
        let p1 = new_parameter(&mut db, "A");
        let p2 = new_parameter(&mut db, "A");
        let p3 = new_parameter(&mut db, "A");

        p2.set_mutable(&mut db);
        p3.set_stack_allocated(&mut db);

        assert_eq!(owned(instance(int)).as_mut(&db), owned(instance(int)));
        assert_eq!(
            uni(instance(int)).as_mut(&db),
            TypeRef::UniMut(instance(int))
        );

        assert_eq!(any(rigid(p1)).as_mut(&db), immutable(rigid(p1)));
        assert_eq!(owned(parameter(p1)).as_mut(&db), mutable(parameter(p1)));

        assert_eq!(owned(rigid(p1)).as_mut(&db), mutable(rigid(p1)));
        assert_eq!(owned(rigid(p2)).as_mut(&db), mutable(rigid(p2)));
        assert_eq!(owned(parameter(p2)).as_mut(&db), mutable(parameter(p2)));
        assert_eq!(owned(instance(ext)).as_mut(&db), pointer(instance(ext)));

        assert_eq!(owned(parameter(p3)).as_mut(&db), owned(parameter(p3)));
        assert_eq!(mutable(parameter(p3)).as_mut(&db), owned(parameter(p3)));
        assert_eq!(uni(parameter(p3)).as_mut(&db), owned(parameter(p3)));
        assert_eq!(owned(rigid(p3)).as_mut(&db), owned(rigid(p3)));
    }

    #[test]
    fn test_type_ref_as_pointer() {
        let mut db = Database::new();
        let int = ClassId::int();
        let ext = new_extern_class(&mut db, "Extern");
        let param = new_parameter(&mut db, "A");
        let var = TypePlaceholder::alloc(&mut db, None);

        assert_eq!(
            owned(instance(int)).as_pointer(&db),
            pointer(instance(int))
        );
        assert_eq!(uni(instance(int)).as_pointer(&db), pointer(instance(int)));
        assert_eq!(owned(rigid(param)).as_pointer(&db), pointer(rigid(param)));
        assert_eq!(
            owned(instance(ext)).as_pointer(&db),
            pointer(instance(ext))
        );
        assert_eq!(
            placeholder(var).as_pointer(&db),
            placeholder(var.as_pointer())
        );
    }

    #[test]
    fn test_type_ref_is_sendable_with_closure() {
        let mut db = Database::new();
        let func1 = Closure::alloc(&mut db, false);
        let func2 = Closure::alloc(&mut db, false);
        let thing = new_class(&mut db, "Thing");
        let var_type = immutable(instance(thing));
        let loc = Location::default();
        let var =
            Variable::alloc(&mut db, "thing".to_string(), var_type, false, loc);

        func2.add_capture(&mut db, var, var_type);

        assert!(owned(closure(func1)).is_sendable(&db));
        assert!(!owned(closure(func2)).is_sendable(&db));
    }

    #[test]
    fn test_type_ref_as_owned_with_placeholder() {
        let mut db = Database::new();
        let var = TypePlaceholder::alloc(&mut db, None);

        assert_eq!(placeholder(var).as_owned(&db), placeholder(var.as_owned()),);
    }

    #[test]
    fn test_type_ref_as_uni_with_placeholder() {
        let mut db = Database::new();
        let var = TypePlaceholder::alloc(&mut db, None);

        assert_eq!(placeholder(var).as_uni(&db), placeholder(var.as_uni()));
    }

    #[test]
    fn test_type_ref_as_ref_with_placeholder() {
        let mut db = Database::new();
        let var = TypePlaceholder::alloc(&mut db, None);

        assert_eq!(placeholder(var).as_ref(&db), placeholder(var.as_ref()));
    }

    #[test]
    fn test_type_ref_as_mut_with_placeholder() {
        let mut db = Database::new();
        let var = TypePlaceholder::alloc(&mut db, None);

        assert_eq!(placeholder(var).as_mut(&db), placeholder(var.as_mut()));
    }

    #[test]
    fn test_type_ref_as_uni_ref_with_placeholder() {
        let mut db = Database::new();
        let var = TypePlaceholder::alloc(&mut db, None);

        assert_eq!(
            placeholder(var).as_uni_ref(&db),
            placeholder(var.as_uni_ref())
        );
    }

    #[test]
    fn test_type_ref_force_as_mut() {
        let mut db = Database::new();
        let p1 = new_parameter(&mut db, "A");
        let p2 = new_parameter(&mut db, "A");

        p2.set_stack_allocated(&mut db);

        assert_eq!(
            owned(parameter(p1)).force_as_mut(&db),
            mutable(parameter(p1))
        );
        assert_eq!(owned(rigid(p1)).force_as_mut(&db), mutable(rigid(p1)));

        assert_eq!(
            owned(parameter(p2)).force_as_mut(&db),
            owned(parameter(p2))
        );
        assert_eq!(
            mutable(parameter(p2)).force_as_mut(&db),
            owned(parameter(p2))
        );
        assert_eq!(uni(parameter(p2)).force_as_mut(&db), owned(parameter(p2)));
        assert_eq!(owned(rigid(p2)).force_as_mut(&db), owned(rigid(p2)));
    }

    #[test]
    fn test_type_ref_force_as_uni_mut_with_placeholder() {
        let mut db = Database::new();
        let var = TypePlaceholder::alloc(&mut db, None);

        assert_eq!(
            placeholder(var).force_as_uni_mut(&db),
            placeholder(var.as_uni_mut())
        );
    }

    #[test]
    fn test_type_ref_as_uni_reference() {
        let mut db = Database::new();
        let foo = new_class(&mut db, "Foo");
        let int = ClassId::int();

        assert_eq!(
            owned(instance(foo)).as_uni_borrow(&db),
            TypeRef::UniMut(instance(foo))
        );
        assert_eq!(
            owned(instance(int)).as_uni_borrow(&db),
            TypeRef::Owned(instance(int))
        );
        assert_eq!(
            immutable(instance(foo)).as_uni_borrow(&db),
            TypeRef::UniRef(instance(foo))
        );
        assert_eq!(
            mutable(instance(foo)).as_uni_borrow(&db),
            TypeRef::UniMut(instance(foo))
        );
        assert_eq!(uni(instance(foo)).as_uni_borrow(&db), uni(instance(foo)));
    }

    #[test]
    fn test_type_ref_as_uni_ref() {
        let mut db = Database::new();
        let foo = new_class(&mut db, "Foo");

        assert_eq!(
            owned(instance(foo)).as_uni_ref(&db),
            immutable_uni(instance(foo))
        );
        assert_eq!(
            mutable(instance(foo)).as_uni_ref(&db),
            immutable_uni(instance(foo))
        );
        assert_eq!(
            uni(instance(foo)).as_uni_ref(&db),
            immutable_uni(instance(foo))
        );
        assert_eq!(
            immutable_uni(instance(foo)).as_uni_ref(&db),
            immutable_uni(instance(foo))
        );
        assert_eq!(
            mutable_uni(instance(foo)).as_uni_ref(&db),
            immutable_uni(instance(foo))
        );
    }

    #[test]
    fn test_type_ref_as_uni_mut() {
        let mut db = Database::new();
        let foo = new_class(&mut db, "Foo");

        assert_eq!(
            owned(instance(foo)).as_uni_mut(&db),
            mutable_uni(instance(foo))
        );
        assert_eq!(
            any(instance(foo)).as_uni_mut(&db),
            immutable_uni(instance(foo))
        );
        assert_eq!(
            mutable(instance(foo)).as_uni_mut(&db),
            mutable_uni(instance(foo))
        );
        assert_eq!(
            uni(instance(foo)).as_uni_mut(&db),
            mutable_uni(instance(foo))
        );
        assert_eq!(
            immutable_uni(instance(foo)).as_uni_mut(&db),
            immutable_uni(instance(foo))
        );
        assert_eq!(
            mutable_uni(instance(foo)).as_uni_mut(&db),
            mutable_uni(instance(foo))
        );
    }

    #[test]
    fn test_type_ref_allow_mutating() {
        let mut db = Database::new();
        let param1 = new_parameter(&mut db, "T");
        let param2 = new_parameter(&mut db, "T");
        let proc = new_async_class(&mut db, "X");

        param2.set_mutable(&mut db);

        assert!(uni(instance(ClassId::string())).allow_mutating(&db));
        assert!(immutable(instance(ClassId::string())).allow_mutating(&db));
        assert!(mutable(parameter(param1)).allow_mutating(&db));
        assert!(mutable(rigid(param1)).allow_mutating(&db));
        assert!(owned(parameter(param1)).allow_mutating(&db));
        assert!(owned(rigid(param1)).allow_mutating(&db));
        assert!(any(parameter(param2)).allow_mutating(&db));
        assert!(any(rigid(param2)).allow_mutating(&db));
        assert!(uni(parameter(param2)).allow_mutating(&db));
        assert!(uni(rigid(param2)).allow_mutating(&db));
        assert!(owned(instance(proc)).allow_mutating(&db));
        assert!(mutable(instance(proc)).allow_mutating(&db));

        assert!(!immutable(instance(proc)).allow_mutating(&db));
        assert!(!immutable(parameter(param1)).allow_mutating(&db));
        assert!(!owned(instance(ClassId::string())).allow_mutating(&db));
        assert!(!any(parameter(param1)).allow_mutating(&db));
        assert!(!any(rigid(param1)).allow_mutating(&db));
        assert!(!TypeRef::int().allow_mutating(&db));
    }

    #[test]
    fn test_type_ref_allow_moving() {
        let mut db = Database::new();
        let heap = new_class(&mut db, "A");
        let stack = new_class(&mut db, "B");

        stack.set_stack_allocated(&mut db);

        assert!(owned(instance(heap)).allow_moving(&db));
        assert!(uni(instance(heap)).allow_moving(&db));
        assert!(owned(instance(stack)).allow_moving(&db));
        assert!(uni(instance(stack)).allow_moving(&db));
        assert!(immutable_uni(instance(stack)).allow_moving(&db));
        assert!(mutable_uni(instance(stack)).allow_moving(&db));
        assert!(!mutable(instance(heap)).allow_moving(&db));
        assert!(!immutable(instance(heap)).allow_moving(&db));
    }

    #[test]
    fn test_module_id_has_same_root_namespace() {
        let mut db = Database::new();
        let foo_mod = Module::alloc(
            &mut db,
            ModuleName::new("std.foo"),
            "foo.inko".into(),
        );

        let bar_mod = Module::alloc(
            &mut db,
            ModuleName::new("std.bar"),
            "bar.inko".into(),
        );

        let bla_mod =
            Module::alloc(&mut db, ModuleName::new("bla"), "bla.inko".into());

        let test_mod = Module::alloc(
            &mut db,
            ModuleName::new("test_bla"),
            "test_bla.inko".into(),
        );

        assert!(foo_mod.has_same_root_namespace(&db, bar_mod));
        assert!(!foo_mod.has_same_root_namespace(&db, bla_mod));
        assert!(bla_mod.has_same_root_namespace(&db, test_mod));
        assert!(!test_mod.has_same_root_namespace(&db, bla_mod));
    }

    #[test]
    fn test_type_ref_type_arguments_with_type_parameter() {
        let mut db = Database::new();
        let trait1 = new_trait(&mut db, "ToA");
        let trait2 = new_trait(&mut db, "ToB");
        let trait3 = new_trait(&mut db, "ToC");
        let param = new_parameter(&mut db, "T");
        let trait1_param = trait1.new_type_parameter(&mut db, "A".to_string());
        let trait2_param = trait2.new_type_parameter(&mut db, "B".to_string());
        let trait3_param = trait3.new_type_parameter(&mut db, "C".to_string());
        let trait1_ins =
            generic_trait_instance(&mut db, trait1, vec![TypeRef::int()]);
        let trait2_ins =
            generic_trait_instance(&mut db, trait2, vec![TypeRef::float()]);
        let trait3_ins =
            generic_trait_instance(&mut db, trait3, vec![TypeRef::string()]);

        trait3.add_required_trait(&mut db, trait2_ins);
        param.add_requirements(&mut db, vec![trait1_ins, trait3_ins]);

        let targs = owned(parameter(param)).type_arguments(&db);

        assert_eq!(targs.get(trait1_param), Some(TypeRef::int()));
        assert_eq!(targs.get(trait2_param), Some(TypeRef::float()));
        assert_eq!(targs.get(trait3_param), Some(TypeRef::string()));
    }

    #[test]
    fn test_type_ref_shape() {
        let mut db = Database::new();
        let mut inter = InternedTypeArguments::new();
        let string = ClassId::string();
        let int = ClassId::int();
        let float = ClassId::float();
        let boolean = ClassId::boolean();
        let cls1 = new_class(&mut db, "Thing");
        let cls2 = new_class(&mut db, "Foo");
        let var = TypePlaceholder::alloc(&mut db, None);
        let param1 = new_parameter(&mut db, "T");
        let param2 = new_parameter(&mut db, "X");
        let mut shapes = HashMap::new();

        cls2.set_stack_allocated(&mut db);
        shapes.insert(param1, Shape::int());
        var.assign(&mut db, TypeRef::int());

        assert_eq!(
            TypeRef::int().shape(&db, &mut inter, &shapes),
            Shape::int()
        );
        assert_eq!(
            TypeRef::float().shape(&db, &mut inter, &shapes),
            Shape::float()
        );
        assert_eq!(
            TypeRef::boolean().shape(&db, &mut inter, &shapes),
            Shape::Boolean
        );
        assert_eq!(TypeRef::nil().shape(&db, &mut inter, &shapes), Shape::Nil);
        assert_eq!(
            TypeRef::string().shape(&db, &mut inter, &shapes),
            Shape::String
        );
        assert_eq!(
            uni(instance(cls1)).shape(&db, &mut inter, &shapes),
            Shape::Owned
        );
        assert_eq!(
            owned(instance(cls1)).shape(&db, &mut inter, &shapes),
            Shape::Owned
        );
        assert_eq!(
            immutable(instance(cls1)).shape(&db, &mut inter, &shapes),
            Shape::Ref
        );
        assert_eq!(
            mutable(instance(cls1)).shape(&db, &mut inter, &shapes),
            Shape::Mut
        );
        assert_eq!(
            uni(instance(cls1)).shape(&db, &mut inter, &shapes),
            Shape::Owned
        );
        assert_eq!(
            placeholder(var).shape(&db, &mut inter, &shapes),
            Shape::int()
        );
        assert_eq!(
            owned(parameter(param1)).shape(&db, &mut inter, &shapes),
            Shape::int()
        );
        assert_eq!(
            immutable(parameter(param1)).shape(&db, &mut inter, &shapes),
            Shape::int()
        );
        assert_eq!(
            mutable(parameter(param1)).shape(&db, &mut inter, &shapes),
            Shape::int()
        );
        assert_eq!(
            owned(TypeId::AtomicTypeParameter(param2))
                .shape(&db, &mut inter, &shapes),
            Shape::Atomic
        );
        assert_eq!(
            immutable(TypeId::AtomicTypeParameter(param2))
                .shape(&db, &mut inter, &shapes),
            Shape::Atomic
        );
        assert_eq!(
            mutable(TypeId::AtomicTypeParameter(param2))
                .shape(&db, &mut inter, &shapes),
            Shape::Atomic
        );

        assert_eq!(
            immutable(instance(string)).shape(&db, &mut inter, &shapes),
            Shape::String
        );
        assert_eq!(
            immutable(instance(int)).shape(&db, &mut inter, &shapes),
            Shape::int()
        );
        assert_eq!(
            immutable(instance(float)).shape(&db, &mut inter, &shapes),
            Shape::float()
        );
        assert_eq!(
            immutable(instance(boolean)).shape(&db, &mut inter, &shapes),
            Shape::Boolean
        );
        assert_eq!(
            mutable(instance(string)).shape(&db, &mut inter, &shapes),
            Shape::String
        );
        assert_eq!(
            mutable(instance(int)).shape(&db, &mut inter, &shapes),
            Shape::int()
        );
        assert_eq!(
            mutable(instance(float)).shape(&db, &mut inter, &shapes),
            Shape::float()
        );
        assert_eq!(
            mutable(instance(boolean)).shape(&db, &mut inter, &shapes),
            Shape::Boolean
        );
        assert_eq!(
            owned(TypeId::Foreign(ForeignType::Int(32, Sign::Signed)))
                .shape(&db, &mut inter, &shapes),
            Shape::Int(32, Sign::Signed)
        );
        assert_eq!(
            owned(TypeId::Foreign(ForeignType::Int(32, Sign::Unsigned)))
                .shape(&db, &mut inter, &shapes),
            Shape::Int(32, Sign::Unsigned)
        );
        assert_eq!(
            uni(TypeId::Foreign(ForeignType::Int(32, Sign::Unsigned)))
                .shape(&db, &mut inter, &shapes),
            Shape::Int(32, Sign::Unsigned)
        );
        assert_eq!(
            owned(TypeId::Foreign(ForeignType::Float(32)))
                .shape(&db, &mut inter, &shapes),
            Shape::Float(32)
        );
        assert_eq!(
            owned(TypeId::Foreign(ForeignType::Float(64)))
                .shape(&db, &mut inter, &shapes),
            Shape::Float(64)
        );
        assert_eq!(
            uni(TypeId::Foreign(ForeignType::Float(64)))
                .shape(&db, &mut inter, &shapes),
            Shape::Float(64)
        );
        assert_eq!(
            pointer(TypeId::Foreign(ForeignType::Int(64, Sign::Signed)))
                .shape(&db, &mut inter, &shapes),
            Shape::Pointer
        );

        assert_eq!(
            owned(instance(cls2)).shape(&db, &mut inter, &shapes),
            Shape::Stack(ClassInstance::new(cls2))
        );
        assert_eq!(
            mutable(instance(cls2)).shape(&db, &mut inter, &shapes),
            Shape::Stack(ClassInstance::new(cls2))
        );
        assert_eq!(
            immutable(instance(cls2)).shape(&db, &mut inter, &shapes),
            Shape::Stack(ClassInstance::new(cls2))
        );
    }

    #[test]
    fn test_type_ref_class_id() {
        let db = Database::new();

        assert_eq!(TypeRef::string().class_id(&db), Some(ClassId::string()));
        assert_eq!(
            owned(TypeId::Class(ClassId::string())).class_id(&db),
            Some(ClassId::string())
        );
    }

    #[test]
    fn test_method_id_receiver_for_class_instance_with_process() {
        let mut db = Database::new();
        let method = Method::alloc(
            &mut db,
            ModuleId(0),
            Location::default(),
            "a".to_string(),
            Visibility::Private,
            MethodKind::Mutable,
        );

        let proc = new_async_class(&mut db, "A");
        let rec =
            method.receiver_for_class_instance(&db, ClassInstance::new(proc));

        assert_eq!(rec, mutable(instance(proc)));
    }

    #[test]
    fn test_type_placeholder_id_as_owned() {
        let id = TypePlaceholderId { id: 1, ownership: Ownership::Any };

        assert_eq!(
            id.as_owned(),
            TypePlaceholderId { id: 1, ownership: Ownership::Owned }
        );
    }

    #[test]
    fn test_type_ref_is_stack_class_instance() {
        let mut db = Database::new();
        let ext = new_extern_class(&mut db, "A");

        assert!(owned(instance(ext)).is_extern_instance(&db));
        assert!(uni(instance(ext)).is_extern_instance(&db));
        assert!(!immutable(instance(ext)).is_extern_instance(&db));
        assert!(!mutable(instance(ext)).is_extern_instance(&db));
        assert!(!pointer(instance(ext)).is_extern_instance(&db));
    }

    #[test]
    fn test_class_id_allow_cast() {
        let mut db = Database::new();
        let enum_class = new_enum_class(&mut db, "Option");
        let regular_class = new_class(&mut db, "Regular");
        let tuple_class = Class::alloc(
            &mut db,
            "Tuple1".to_string(),
            ClassKind::Tuple,
            Visibility::Public,
            ModuleId(0),
            Location::default(),
        );

        assert!(!ClassId::int().allow_cast_to_trait(&db));
        assert!(!ClassId::float().allow_cast_to_trait(&db));
        assert!(!ClassId::boolean().allow_cast_to_trait(&db));
        assert!(!ClassId::nil().allow_cast_to_trait(&db));
        assert!(!ClassId::string().allow_cast_to_trait(&db));
        assert!(enum_class.allow_cast_to_trait(&db));
        assert!(tuple_class.allow_cast_to_trait(&db));
        assert!(regular_class.allow_cast_to_trait(&db));
    }

    #[test]
    fn test_interned_type_arguments() {
        let mut db = Database::new();
        let mut intern = InternedTypeArguments::new();
        let ary = ClassId::array();
        let int = TypeRef::int();
        let param = ary.new_type_parameter(&mut db, "T".to_string());
        let val1 = {
            let sub = owned(generic_instance_id(&mut db, ary, vec![int]));

            owned(generic_instance_id(&mut db, ary, vec![sub]))
        };
        let val2 = {
            let sub = owned(generic_instance_id(&mut db, ary, vec![int]));

            owned(generic_instance_id(&mut db, ary, vec![sub]))
        };
        let mut targs1 = TypeArguments::new();
        let mut targs2 = TypeArguments::new();

        targs1.assign(param, val1);
        targs2.assign(param, val2);

        let ins1 = ClassInstance::generic(&mut db, ary, targs1);
        let ins2 = ClassInstance::generic(&mut db, ary, targs2);
        let id1 = intern.intern(&db, ins1);
        let id2 = intern.intern(&db, ins2);
        let id3 = intern.intern(&db, ins1);
        let id4 = intern.intern(&db, ins2);

        assert_eq!(id1, ins1.type_arguments);
        assert_eq!(id2, id1);
        assert_eq!(id3, id1);
        assert_eq!(id4, id1);
    }
}
