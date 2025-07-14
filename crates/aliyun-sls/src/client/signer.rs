use crate::client::headers;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use hmac::{Hmac, Mac};
use jiff::Timestamp;
use sha1::Sha1;

pub(super) struct Signer {
    pub(super) hmac: Hmac<Sha1>,
    pub(super) access_key: String,
    pub(super) canonicalized_resource: String,
}

pub(super) struct Signature {
    pub(super) date: String,
    pub(super) raw_length: String,
    pub(super) content_md5: String,
    pub(super) authorization: String,
}

impl Signer {
    pub fn sign(&self, encoded_len: usize, encoded: &[u8]) -> Signature {
        let mut mac = self.hmac.clone();

        let date = Timestamp::now()
            .strftime("%a, %d %b %Y %H:%M:%S GMT")
            .to_string();
        let raw_length = encoded_len.to_string();
        let content_md5 = hex::encode_upper(md5::compute(encoded).as_ref());

        // SignString = VERB + "\n"
        //     + CONTENT-MD5 + "\n"
        //     + CONTENT-TYPE + "\n"
        //     + DATE + "\n"
        //     + CanonicalizedLOGHeaders + "\n"
        //     + CanonicalizedResource
        mac.update(b"POST\n");
        mac.update(content_md5.as_bytes());
        mac.update(b"\n");

        mac.update(headers::DEFAULT_CONTENT_TYPE.as_bytes());
        mac.update(b"\n");

        mac.update(date.as_bytes());
        mac.update(b"\n");

        // CanonicalizedLOGHeaders的构造方式如下：
        // 将所有以x-log和x-acs为前缀的HTTP请求头的名字转换成小写字母。
        // 将上一步得到的所有LOG自定义请求头按照字典顺序进行升序排序。
        // 删除请求头和内容之间分隔符两端出现的任何空格。
        // 将所有的头和内容用\n分隔符组合成最后的CanonicalizedLOGHeader。
        mac.update(headers::LOG_API_VERSION.as_bytes());
        mac.update(b":");
        mac.update(headers::API_VERSION.as_bytes());
        mac.update(b"\n");
        mac.update(headers::LOG_BODY_RAW_SIZE.as_bytes());
        mac.update(b":");
        mac.update(raw_length.as_bytes());
        #[cfg(not(any(feature = "lz4", feature = "deflate")))]
        mac.update(b"\n");
        #[cfg(feature = "lz4")]
        mac.update(b"\nx-log-compresstype:lz4\n");
        #[cfg(feature = "deflate")]
        mac.update(b"\nx-log-compresstype:deflate\n");
        mac.update(headers::LOG_SIGNATURE_METHOD.as_bytes());
        mac.update(b":");
        mac.update(headers::SIGNATURE_METHOD.as_bytes());
        mac.update(b"\n");

        // CanonicalizedResource的构造方式如下：
        // a. 将CanonicalizedResource设置为空字符串" "。
        // b. 放入要访问的LOG资源，如/logstores/logstorename（如果没有logstorename则可不填写）。
        // c. 如果请求包含查询字符串QUERY_STRING，则在CanonicalizedResource字符串尾部添加?和查询字符串。
        //
        // QUERY_STRING是URL中请求参数按字典顺序排序后的字符串，其中参数名和值之间用=相隔组成字符串，并对参数名-值对按照字典顺序升序排序，然后以&符号连接构成字符串。其公式化描述如下：
        // QUERY_STRING = "KEY1=VALUE1" + "&" + "KEY2=VALUE2"
        mac.update(self.canonicalized_resource.as_bytes());
        let authorization = BASE64_STANDARD.encode(mac.finalize().into_bytes());
        let authorization = format!("LOG {}:{}", self.access_key, authorization);

        Signature {
            date,
            raw_length,
            content_md5,
            authorization,
        }
    }
}
