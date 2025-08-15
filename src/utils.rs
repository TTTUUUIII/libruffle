use jni::{objects::JString, JNIEnv};

pub struct JniUtils;

impl JniUtils{
    pub fn to_string(env: &mut JNIEnv, s: JString) -> String {
        let it = env
            .get_string(&s)
            .expect("Failed to get String from JString!");
        it.into()
    }
}