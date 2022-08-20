use crate::{
    avm1::SoundObject,
    display_object::{
        self, DisplayObject, MovieClip, SoundTransform as DisplayObjectSoundTransform,
        TDisplayObject,
    },
};
use downcast_rs::Downcast;
use gc_arena::Collect;
use generational_arena::{Arena, Index};

pub mod decoders;
pub mod swf {
    pub use swf::{
        read, AudioCompression, CharacterId, Sound, SoundEnvelope, SoundEnvelopePoint, SoundEvent,
        SoundFormat, SoundInfo, SoundStreamHead,
    };
}

pub type SoundHandle = Index;
pub type SoundInstanceHandle = Index;
pub type PreloadStreamHandle = u32;

type Error = Box<dyn std::error::Error>;

pub trait AudioBackend: Downcast {
    fn play(&mut self);
    fn pause(&mut self);
    fn register_sound(&mut self, swf_sound: &swf::Sound) -> Result<SoundHandle, Error>;

    /// Used by the web backend to pre-decode sound streams.
    /// Returns the sound handle to be used to add data to the stream.
    /// Other backends return `None`.
    /// TODO: Get rid of the preload_* methods when web backend has a better way
    /// of decoding audio on the fly.
    fn preload_sound_stream_head(
        &mut self,
        _stream_info: &swf::SoundStreamHead,
    ) -> Option<PreloadStreamHandle> {
        None
    }

    /// Used by the web backend to add data to a currently preloading sound stream.
    fn preload_sound_stream_block(
        &mut self,
        _stream: PreloadStreamHandle,
        _clip_frame: u16,
        _audio_data: &[u8],
    ) {
    }

    /// Used by the web backend to finalize and decode a sound stream.
    /// Returns true if this was a valid stream.
    fn preload_sound_stream_end(&mut self, _stream: PreloadStreamHandle) -> Option<SoundHandle> {
        None
    }

    /// Plays a sound.
    fn start_sound(
        &mut self,
        sound: SoundHandle,
        settings: &swf::SoundInfo,
    ) -> Result<SoundInstanceHandle, Error>;

    /// Starts playing a "stream" sound, which is an audio stream that is distributed
    /// among the frames of a Flash MovieClip.
    /// On the web backend, `stream_handle` should be the handle for the preloaded stream.
    /// Other backends can pass `None`.
    fn start_stream(
        &mut self,
        stream_handle: Option<SoundHandle>,
        clip_frame: u16,
        clip_data: crate::tag_utils::SwfSlice,
        handle: &swf::SoundStreamHead,
    ) -> Result<SoundInstanceHandle, Error>;

    /// Stops a playing sound instance.
    /// No-op if the sound is not playing.
    fn stop_sound(&mut self, sound: SoundInstanceHandle);

    /// Good ol' stopAllSounds() :-)
    fn stop_all_sounds(&mut self);

    /// Get the position of a sound instance in milliseconds.
    /// Returns `None` if ther sound is not/no longer playing
    fn get_sound_position(&self, instance: SoundInstanceHandle) -> Option<u32>;

    /// Get the duration of a sound in milliseconds.
    /// Returns `None` if sound is not registered.
    fn get_sound_duration(&self, sound: SoundHandle) -> Option<u32>;

    /// Set the volume transform for a sound instance.
    fn set_sound_transform(&mut self, instance: SoundInstanceHandle, transform: SoundTransform);

    // TODO: Eventually remove this/move it to library.
    fn is_loading_complete(&self) -> bool {
        true
    }
    fn tick(&mut self) {}

    /// Inform the audio backend of the current stage frame rate.
    ///
    /// This is only necessary if your particular audio backend needs to know
    /// what the stage frame rate is. Otherwise, you are free to avoid
    /// implementing it.
    fn set_frame_rate(&mut self, _frame_rate: f64) {}
}

impl_downcast!(AudioBackend);

/// Audio backend that ignores all audio.
pub struct NullAudioBackend {
    sounds: Arena<()>,
}

impl NullAudioBackend {
    pub fn new() -> NullAudioBackend {
        NullAudioBackend {
            sounds: Arena::new(),
        }
    }
}

impl AudioBackend for NullAudioBackend {
    fn play(&mut self) {}
    fn pause(&mut self) {}
    fn register_sound(&mut self, _sound: &swf::Sound) -> Result<SoundHandle, Error> {
        Ok(self.sounds.insert(()))
    }

    fn start_sound(
        &mut self,
        _sound: SoundHandle,
        _sound_info: &swf::SoundInfo,
    ) -> Result<SoundInstanceHandle, Error> {
        Ok(SoundInstanceHandle::from_raw_parts(0, 0))
    }

    fn start_stream(
        &mut self,
        _stream_handle: Option<SoundHandle>,
        _clip_frame: u16,
        _clip_data: crate::tag_utils::SwfSlice,
        _handle: &swf::SoundStreamHead,
    ) -> Result<SoundInstanceHandle, Error> {
        Ok(SoundInstanceHandle::from_raw_parts(0, 0))
    }

    fn stop_sound(&mut self, _sound: SoundInstanceHandle) {}

    fn stop_all_sounds(&mut self) {}
    fn get_sound_position(&self, _instance: SoundInstanceHandle) -> Option<u32> {
        None
    }
    fn get_sound_duration(&self, _sound: SoundHandle) -> Option<u32> {
        None
    }

    fn set_sound_transform(&mut self, _instance: SoundInstanceHandle, _transform: SoundTransform) {}
}

impl Default for NullAudioBackend {
    fn default() -> Self {
        NullAudioBackend::new()
    }
}

#[derive(Collect)]
#[collect(no_drop)]
pub struct AudioManager<'gc> {
    /// The list of actively playing sounds.
    sounds: Vec<SoundInstance<'gc>>,

    /// The global sound transform applied to all sounds.
    global_sound_transform: DisplayObjectSoundTransform,

    /// Whether a sound transform has been changed.
    transforms_dirty: bool,
}

impl<'gc> AudioManager<'gc> {
    /// The maximum number of sound instances that can play at once.
    pub const MAX_SOUNDS: usize = 32;

    pub fn new() -> Self {
        Self {
            sounds: Vec::with_capacity(Self::MAX_SOUNDS),
            global_sound_transform: Default::default(),
            transforms_dirty: false,
        }
    }

    /// Update state of active sounds. Should be called once per frame.
    pub fn update_sounds(
        &mut self,
        audio: &mut dyn AudioBackend,
        gc_context: gc_arena::MutationContext<'gc, '_>,
        action_queue: &mut crate::context::ActionQueue<'gc>,
        root: DisplayObject<'gc>,
    ) {
        // Update the position of sounds, and remove any completed sounds.
        self.sounds.retain(|sound| {
            if let Some(pos) = audio.get_sound_position(sound.instance) {
                // Sounds still playing; update position.
                if let Some(avm1_object) = sound.avm1_object {
                    avm1_object.set_position(gc_context, pos);
                }
                true
            } else {
                // Sound ended; fire end event.
                if let Some(object) = sound.avm1_object {
                    action_queue.queue_actions(
                        root,
                        crate::context::ActionType::Method {
                            object: object.into(),
                            name: "onSoundComplete",
                            args: vec![],
                        },
                        false,
                    );
                }
                false
            }
        });

        // Update sound transforms, if dirty.
        self.update_sound_transforms(audio);
    }

    pub fn start_sound(
        &mut self,
        audio: &mut dyn AudioBackend,
        sound: SoundHandle,
        settings: &swf::SoundInfo,
        display_object: Option<DisplayObject<'gc>>,
        avm1_object: Option<SoundObject<'gc>>,
    ) -> Option<SoundInstanceHandle> {
        if self.sounds.len() < Self::MAX_SOUNDS {
            let handle = audio.start_sound(sound, settings).ok()?;
            let instance = SoundInstance {
                sound: Some(sound),
                instance: handle,
                display_object,
                avm1_object,
            };
            audio.set_sound_transform(handle, self.transform_for_sound(&instance));
            self.sounds.push(instance);
            Some(handle)
        } else {
            None
        }
    }

    pub fn stop_sound(&mut self, audio: &mut dyn AudioBackend, instance: SoundInstanceHandle) {
        if let Some(i) = self
            .sounds
            .iter()
            .position(|other| other.instance == instance)
        {
            let instance = &self.sounds[i];
            audio.stop_sound(instance.instance);
            self.sounds.swap_remove(i);
        }
    }

    pub fn stop_sounds_with_handle(&mut self, audio: &mut dyn AudioBackend, sound: SoundHandle) {
        self.sounds.retain(move |other| {
            if other.sound == Some(sound) {
                audio.stop_sound(other.instance);
                false
            } else {
                true
            }
        });
    }

    pub fn stop_sounds_with_display_object(
        &mut self,
        audio: &mut dyn AudioBackend,
        display_object: DisplayObject<'gc>,
    ) {
        self.sounds.retain(move |sound| {
            if let Some(other) = sound.display_object {
                if DisplayObject::ptr_eq(other, display_object) {
                    audio.stop_sound(sound.instance);
                    return false;
                }
            }
            true
        });
    }

    pub fn stop_all_sounds(&mut self, audio: &mut dyn AudioBackend) {
        self.sounds.clear();
        audio.stop_all_sounds();
    }

    pub fn is_sound_playing_with_handle(&mut self, sound: SoundHandle) -> bool {
        self.sounds.iter().any(|other| other.sound == Some(sound))
    }

    pub fn start_stream(
        &mut self,
        audio: &mut dyn AudioBackend,
        stream_handle: Option<SoundHandle>,
        movie_clip: MovieClip<'gc>,
        clip_frame: u16,
        data: crate::tag_utils::SwfSlice,
        stream_info: &swf::SoundStreamHead,
    ) -> Option<SoundInstanceHandle> {
        if self.sounds.len() < Self::MAX_SOUNDS {
            let handle = audio
                .start_stream(stream_handle, clip_frame, data, stream_info)
                .ok()?;
            let instance = SoundInstance {
                sound: None,
                instance: handle,
                display_object: Some(movie_clip.into()),
                avm1_object: None,
            };
            audio.set_sound_transform(handle, self.transform_for_sound(&instance));
            self.sounds.push(instance);
            Some(handle)
        } else {
            None
        }
    }

    pub fn global_sound_transform(&self) -> &DisplayObjectSoundTransform {
        &self.global_sound_transform
    }

    pub fn set_global_sound_transform(&mut self, sound_transform: DisplayObjectSoundTransform) {
        self.global_sound_transform = sound_transform;
        self.transforms_dirty = true;
    }

    pub fn set_sound_transforms_dirty(&mut self) {
        self.transforms_dirty = true;
    }

    fn transform_for_sound(&self, sound: &SoundInstance<'gc>) -> SoundTransform {
        let mut transform = DisplayObjectSoundTransform::default();
        let mut parent = sound.display_object;
        while let Some(display_object) = parent {
            transform.concat(&display_object.sound_transform());
            parent = display_object.parent();
        }
        transform.concat(&self.global_sound_transform);
        SoundTransform::from_display_object_transform(&transform)
    }

    /// Update the sound transforms for all sounds.
    /// This should be called whenever a sound transform changes on a display object.
    fn update_sound_transforms(&mut self, audio: &mut dyn AudioBackend) {
        // This updates the sound transform for all sounds, even though the transform has
        // only changed on a single display object. There are only a small amount
        // of sounds playing at any time, so this shouldn't be a big deal.
        if self.transforms_dirty {
            for sound in &self.sounds {
                let transform = self.transform_for_sound(sound);
                audio.set_sound_transform(sound.instance, transform);
            }
            self.transforms_dirty = false;
        }
    }
}

impl<'gc> Default for AudioManager<'gc> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Collect)]
#[collect(no_drop)]
pub struct SoundInstance<'gc> {
    /// The handle to the sound instance in the audio backend.
    #[collect(require_static)]
    instance: SoundInstanceHandle,

    /// The handle to the sound definition in the audio backend.
    /// This will be `None` for stream sounds.
    #[collect(require_static)]
    sound: Option<SoundHandle>,

    /// The display object that this sound is playing in, if any.
    /// Used for volume mixing and `Sound.stop()`.
    display_object: Option<DisplayObject<'gc>>,

    /// The AVM1 `Sound` object associated with this sound, if any.
    pub avm1_object: Option<SoundObject<'gc>>,
}

/// A sound transform for a playing sound, for use by audio backends.
/// This differs from `display_object::SoundTranform` by being
/// already converted to `f32` and having `volume` baked in.
#[derive(Debug, PartialEq, Clone)]
pub struct SoundTransform {
    pub left_to_left: f32,
    pub left_to_right: f32,
    pub right_to_left: f32,
    pub right_to_right: f32,
}

impl SoundTransform {
    /// Converts from a `display_object::SoundTransform` to a `backend::audio::SoundTransform`.
    fn from_display_object_transform(other: &DisplayObjectSoundTransform) -> Self {
        const SCALE: f32 = (display_object::SoundTransform::MAX_VOLUME
            * display_object::SoundTransform::MAX_VOLUME) as f32;
        Self {
            left_to_left: other.left_to_left as f32 * other.volume as f32 / SCALE,
            left_to_right: other.left_to_right as f32 * other.volume as f32 / SCALE,
            right_to_left: other.right_to_left as f32 * other.volume as f32 / SCALE,
            right_to_right: other.right_to_right as f32 * other.volume as f32 / SCALE,
        }
    }
}

impl Default for SoundTransform {
    fn default() -> Self {
        Self {
            left_to_left: 1.0,
            left_to_right: 0.0,
            right_to_left: 0.0,
            right_to_right: 1.0,
        }
    }
}
