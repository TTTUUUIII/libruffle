use std::collections::HashMap;
use jni::{objects::{JObject, JString}, JNIEnv};

pub struct JniUtils;

impl JniUtils{
    pub fn to_string(env: &mut JNIEnv, s: JString) -> String {
        let it = env
            .get_string(&s)
            .expect("Failed to case JString as String");
        it.into()
    }

    pub fn as_float(env: &mut JNIEnv, val: JObject) -> f32 {
        env.call_method(val, "floatValue", "()F", &[])
            .expect("Failed to case JObject as f32")
            .f()
            .expect("Failed to case JObject as f32")
    }

    pub fn as_string(env: &mut JNIEnv, val: JObject) -> String {

        let clazz = env.find_class("java/io/File").unwrap();
        if env.is_instance_of(&val, clazz).is_ok() {
            let obj = env.call_method(val, "getAbsolutePath", "()Ljava/lang/String;", &[])
                .expect("Failed to case JObject as String")
                .l()
                .expect("Failed to case JObject as String");
            let s = JString::from(obj);
            env.get_string(&s)
                .expect("Failed to case JObject as String")
                .into()
        } else {
            let s = JString::from(val);
            env.get_string(&s)
                .expect("Failed as JObject to String")
                .into()   
        }
    }
}



#[derive(Clone, Debug)]
pub enum TypedValue {
    I(i32),
    F(f32),
    S(String)
}

pub struct Properties {
    data_opt: Option<HashMap<String, TypedValue>>
}

impl Properties {
    pub const fn new() -> Self {
        Self { data_opt: None }
    }

    pub fn put(&mut self, key: &str, v: TypedValue) {
        if self.data_opt.is_none() {
            self.data_opt = Some(HashMap::new());
        }
        self.data_opt
            .as_mut()
            .unwrap()
            .insert(key.to_string(), v);
    }

    pub fn f(&mut self, key: &str, def_value: f32) -> f32 {
        if self.data_opt.is_none() {
            def_value
        } else {
            match self.data_opt
                .as_ref()
                .unwrap()
                .get(key) {
                Some(TypedValue::F(v)) => *v,
                _ => def_value,
            }
        }
    }

    pub fn s(&mut self, key: &str) -> Option<&String> {
        if self.data_opt.is_none() {
            None
        } else {
            match self.data_opt.as_ref().unwrap().get(key) {
                Some(TypedValue::S(v)) => Some(v),
                _ => None
            }
        }
    }
}