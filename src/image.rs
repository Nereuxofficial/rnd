use zbus::zvariant::{OwnedValue, Structure, Value};
use zbus::Error;

#[allow(unused)]
#[derive(Debug)]
pub struct Image {
    pub width: i32,
    pub height: i32,
    pub rowstride: i32,
    pub has_alpha: bool,
    pub bits_per_sample: i32,
    pub channels: i32,
    // The Pixels, converted to RBGA
    pub pixels: Vec<u8>,
}

fn rbg_to_rgba(rgb: Vec<u8>) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(rgb.len() * 4 / 3);
    for chunk in rgb.chunks(3) {
        rgba.extend_from_slice(chunk);
        rgba.push(255);
    }
    rgba
}

impl TryFrom<OwnedValue> for Image {
    type Error = Error;

    fn try_from(value: zbus::zvariant::OwnedValue) -> Result<Self, Self::Error> {
        let res: Result<Structure, _> = Structure::try_from(value.try_clone()?);
        if let Ok(structure) = res {
            let mut field_iter = structure.fields().iter();
            let width = i32::try_from(field_iter.next().unwrap().clone())?;
            let height = i32::try_from(field_iter.next().unwrap().clone())?;
            let rowstride = i32::try_from(field_iter.next().unwrap().clone())?;
            let has_alpha = bool::try_from(field_iter.next().unwrap().clone())?;
            let bits_per_sample = i32::try_from(field_iter.next().unwrap().clone())?;
            let channels = i32::try_from(field_iter.next().unwrap().clone())?;
            let pixels: Vec<_> = field_iter
                .next()
                .map(|f| match f {
                    Value::Array(a) => Some(a),
                    _ => None,
                })
                .ok_or(Error::Failure(
                    "No raw image data found as defined by the spec".to_string(),
                ))?
                .map(|a| a.iter().map(|v| u8::try_from(v.clone()).unwrap()))
                .unwrap()
                .collect();

            Ok(Self {
                width,
                height,
                rowstride,
                has_alpha,
                bits_per_sample,
                channels,
                pixels: rbg_to_rgba(pixels),
            })
        } else {
            Err(Error::Failure("Image Data not valid".to_string()))
        }
    }
}
