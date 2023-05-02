use crate::generate::*;
use wit_parser::*;

pub struct DartType {
    pub name: String,
    pub ty: Type,
    pub ffi_ty: String,
    pub is_pointer: bool,
}

pub enum FuncKind {
    MethodCall,
    Method,
    Field,
}

pub struct Parsed<'a>(pub &'a UnresolvedPackage);

impl Parsed<'_> {
    pub fn type_to_ffi(&self, ty: &Type) -> String {
        match ty {
            Type::Id(ty_id) => {
                let ty_def = self.0.types.get(*ty_id).unwrap();
                ty_def.name.clone().unwrap()
            }
            Type::Bool => "Bool".to_string(),
            Type::String => "String".to_string(),
            Type::Char => "Uint32".to_string(),
            Type::Float32 => "Float".to_string(),
            Type::Float64 => "Double".to_string(),
            Type::S8 => "Int8".to_string(),
            Type::S16 => "Int16".to_string(),
            Type::S32 => "Int32".to_string(),
            Type::S64 => "Int64".to_string(),
            Type::U8 => "Uint8".to_string(),
            Type::U16 => "Uint16".to_string(),
            Type::U32 => "Uint32".to_string(),
            Type::U64 => "Uint64".to_string(),
            // Type::USize => "usize".to_string(),
            // Type::Alias(alias) => alias.type_.ffi_type(),
            // Type::Handle(_resource_name) => self.as_lang(),
            // Type::ConstPtr(_pointee) => _pointee.as_lang(),
            // Type::MutPtr(_pointee) => _pointee.as_lang(),
            // Type::Option(_) => todo!(),
            // Type::Result(_) => todo!(),
            // Type::Void => "Void".to_string(),
        }
    }

    pub fn type_to_str(&self, ty: &Type) -> String {
        match ty {
            Type::Id(ty_id) => {
                let ty_def = self.0.types.get(*ty_id).unwrap();
                self.type_def_to_name(ty_def)
            }
            Type::Bool => "bool".to_string(),
            Type::String => "String".to_string(),
            Type::Char => "int /* Char */".to_string(),
            Type::Float32 => "double /* Float32 */".to_string(),
            Type::Float64 => "double /* Float64 */".to_string(),
            Type::S8 => "int /* S8 */".to_string(),
            Type::S16 => "int /* S16 */".to_string(),
            Type::S32 => "int /* S32 */".to_string(),
            Type::S64 => "int /* S64 */".to_string(),
            Type::U8 => "int /* U8 */".to_string(),
            Type::U16 => "int /* U16 */".to_string(),
            Type::U32 => "int /* U32 */".to_string(),
            Type::U64 => "int /* U64 */".to_string(),
        }
    }

    pub fn type_to_dart_definition(&self, ty: &Type) -> String {
        match ty {
            Type::Id(ty_id) => {
                let ty_def = self.0.types.get(*ty_id).unwrap();
                self.type_def_to_definition(ty_def)
            }
            Type::Bool => "".to_string(),
            Type::String => "".to_string(),
            Type::Char => "".to_string(),
            Type::Float32 => "".to_string(),
            Type::Float64 => "".to_string(),
            Type::S8 => "".to_string(),
            Type::S16 => "".to_string(),
            Type::S32 => "".to_string(),
            Type::S64 => "".to_string(),
            Type::U8 => "".to_string(),
            Type::U16 => "".to_string(),
            Type::U32 => "".to_string(),
            Type::U64 => "".to_string(),
        }
    }

    pub fn type_def_to_name(&self, ty: &TypeDef) -> String {
        let name = ty.name.as_ref().map(heck::AsPascalCase);
        match &ty.kind {
            TypeDefKind::Record(_record) => name.unwrap().to_string(),
            TypeDefKind::Enum(_enum) => name.unwrap().to_string(),
            TypeDefKind::Union(_union) => name.unwrap().to_string(),
            TypeDefKind::Flags(_flags) => name.unwrap().to_string(),
            TypeDefKind::Variant(_variant) => name.unwrap().to_string(),
            TypeDefKind::Tuple(t) => {
                format!(
                    "({})",
                    t.types
                        .iter()
                        .map(|t| self.type_to_ffi(t))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            TypeDefKind::Option(ty) => format!("Option<{}>", self.type_to_str(&ty)),
            TypeDefKind::Result(r) => format!(
                "Result<{}, {}>",
                r.ok.map(|ty| self.type_to_str(&ty))
                    .unwrap_or("void".to_string()),
                r.err
                    .map(|ty| self.type_to_str(&ty))
                    .unwrap_or("void".to_string())
            ),
            TypeDefKind::List(ty) => format!("List<{}>", self.type_to_str(&ty)),
            TypeDefKind::Future(ty) => format!(
                "Future<{}>",
                ty.map(|ty| self.type_to_str(&ty))
                    .unwrap_or("void".to_string())
            ),
            TypeDefKind::Stream(s) => format!(
                "Stream<{}>",
                // TODO: stream.end
                s.element
                    .map(|ty| self.type_to_str(&ty))
                    .unwrap_or("void".to_string()),
            ),
            TypeDefKind::Type(ty) => self.type_to_str(&ty),
            TypeDefKind::Unknown => unimplemented!("Unknown type"),
        }
    }

    pub fn type_def_to_definition(&self, ty: &TypeDef) -> String {
        let name = ty.name.as_ref().map(heck::AsPascalCase);

        let mut s = String::new();
        add_docs(&mut s, &ty.docs);
        match &ty.kind {
            TypeDefKind::Record(r) => {
                let name = name.unwrap();
                s.push_str(&format!("class {} {{", name));
                r.fields.iter().for_each(|f| {
                    add_docs(&mut s, &f.docs);
                    s.push_str(&format!("final {} {};", self.type_to_str(&f.ty), f.name));
                });
                s.push_str(&format!("const {}({{", name));
                r.fields.iter().for_each(|f| {
                    s.push_str(&format!("required this.{},", f.name));
                });
                s.push_str("});}");
                s
            }
            TypeDefKind::Enum(e) => {
                s.push_str(&format!("enum {} {{", name.unwrap()));
                e.cases.iter().for_each(|v| {
                    add_docs(&mut s, &v.docs);
                    s.push_str(&format!("{},", heck::AsLowerCamelCase(&v.name)));
                });
                s.push_str("}");
                s
            }
            TypeDefKind::Union(u) => {
                let name = name.unwrap();
                s.push_str(&format!("sealed class {} {{}}", name));

                u.cases.iter().for_each(|v| {
                    add_docs(&mut s, &v.docs);
                    let ty = self.type_to_str(&v.ty);
                    let inner_name = heck::AsPascalCase(&ty);
                    s.push_str(&format!(
                        "class {}{} implements {} {{ final {} value; const {}{}(this.value); }}",
                        name, inner_name, name, ty, name, inner_name
                    ));
                });
                s
            }
            TypeDefKind::Variant(a) => {
                let name = name.unwrap();
                s.push_str(&format!("sealed class {} {{}}", name));
                a.cases.iter().for_each(|v| {
                    add_docs(&mut s, &v.docs);
                    let inner_name =  heck::AsPascalCase(&v.name);
                    if let Some(ty) = v.ty {
                        let ty =self.type_to_str(&ty);
                        s.push_str(&format!(
                            "class {}{} implements {} {{ final {} value; const {}{}(this.value); }}",
                            name, inner_name, name, ty, name, inner_name
                        ));
                    } else {
                        s.push_str(&format!(
                            "class {}{} implements {} {{ const {}{}(); }}",
                            name, inner_name, name, name, inner_name
                        ));
                    }
                });
                s
            }
            TypeDefKind::Flags(f) => {
                let name = name.unwrap();
                s.push_str(&format!("typedef {} = int; class {}Flag {{", name, name));
                f.flags.iter().enumerate().for_each(|(i, v)| {
                    add_docs(&mut s, &v.docs);
                    // TODO: proper representation of flags
                    s.push_str(&format!("static const {} = {};", v.name, i));
                });
                s.push_str("}");
                s
            }
            TypeDefKind::Type(ty) => self.type_to_dart_definition(ty),
            TypeDefKind::List(_) => s,
            TypeDefKind::Tuple(_) => s,
            TypeDefKind::Option(_) => s,
            TypeDefKind::Result(_) => s,
            TypeDefKind::Future(_) => s,
            TypeDefKind::Stream(_) => s,
            TypeDefKind::Unknown => todo!(),
        }
    }

    pub fn add_interfaces(
        &self,
        mut s: &mut String,
        map: &mut dyn Iterator<Item = (&String, &WorldItem)>,
    ) {
        map.for_each(|(id, item)| match item {
            WorldItem::Interface(interface_id) => {
                let interface = self.0.interfaces.get(*interface_id).unwrap();
                self.add_interface(&mut s, &heck::AsPascalCase(id).to_string(), interface)
            }
            _ => {}
        });
    }

    pub fn add_interface(&self, mut s: &mut String, name: &str, interface: &Interface) {
        add_docs(&mut s, &interface.docs);
        s.push_str(&format!(
            "class {} {{",
            name, // interface.name.as_ref().unwrap())
        ));
        interface.functions.iter().for_each(|(id, f)| {
            self.add_function(&mut s, f, FuncKind::Method);
        });
        s.push_str("}");
    }

    pub fn add_function(&self, mut s: &mut String, f: &Function, kind: FuncKind) {
        let params = f
            .params
            .iter()
            .map(|(name, ty)| format!("{} {}", self.type_to_str(ty), name))
            .collect::<Vec<_>>()
            .join(",");

        let mut results = match &f.results {
            Results::Anon(ty) => self.type_to_str(ty),
            Results::Named(list) => list
                .iter()
                .map(|(name, ty)| format!("{} {}", self.type_to_str(ty), name))
                .collect::<Vec<_>>()
                .join(","),
        };
        if results.is_empty() {
            results = "void".to_string();
        }

        add_docs(&mut s, &f.docs);
        match kind {
            FuncKind::Field => s.push_str(&format!(
                "final {} Function({}) {};",
                results, params, f.name
            )),
            FuncKind::Method => s.push_str(&format!("{} {}({});", results, f.name, params,)),
            FuncKind::MethodCall => {
                s.push_str(&format!("late final _{} = lookup('{}');", f.name, f.name));
                s.push_str(&format!("{} {}({}) {{", results, f.name, params,));
                s.push_str(&format!(
                    "return _{}({});",
                    f.name,
                    f.params
                        .iter()
                        .map(|(name, _)| name.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                s.push_str("}");
            }
        }
    }
}