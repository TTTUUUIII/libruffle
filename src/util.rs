use std::collections::HashMap;
use once_cell::sync::Lazy;
use jni::{objects::{JObject, JString}, JNIEnv};

pub struct JniUtils;

impl JniUtils{
    pub fn to_string(env: &mut JNIEnv, s: JString) -> String {
        let it = env
            .get_string(&s)
            .expect("Failed to get String from JString!");
        it.into()
    }

    pub fn to_float(env: &mut JNIEnv, val: JObject) -> f32 {
        env.call_method(val, "floatValue", "()F", &[])
            .expect("Failed to call method => Float::floatValue()")
            .f()
            .expect("Expected float return")
    }
}



#[derive(Clone, Debug)]
enum Type {
    I(i32),
    F(f32),
    S(String)
}

pub struct Properties {
    props: Lazy<HashMap<String, Type>>
}

impl Properties {
    pub const fn new() -> Self {
        Self { props: Lazy::new(||{HashMap::new()}) }
    }

    pub fn set_prop(&mut self, key: &str, val: f32) {
        self.props.insert(key.to_string(), Type::F(val));
    }

    pub fn get_float(&self, k: &str, def_v: f32) -> f32 {
        let key = k.to_string();
        if let Some(Type::F(val)) = self.props.get(&key) {
            val.clone()
        } else {
            def_v
        }
    }
}