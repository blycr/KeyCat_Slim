#ifndef KEYCAT_FFI_H
#define KEYCAT_FFI_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct KcScene KcScene;
typedef struct KcAudio KcAudio;

/* Hit zones */
enum {
    KC_HIT_NONE = 0,
    KC_HIT_PET  = 1,
    KC_HIT_DRAG = 2,
};

/* Pipeline kinds */
enum {
    KC_PIPE_BASIC     = 0,
    KC_PIPE_FUR       = 1,
    KC_PIPE_INSTANCED = 2,
};

/* Cull modes (front face is always CCW) */
enum {
    KC_CULL_BACK  = 0,
    KC_CULL_FRONT = 1,
    KC_CULL_NONE  = 2,
    KC_CULL_DECAL = 3,
};

/* Depth modes */
enum {
    KC_DEPTH_RW   = 0,
    KC_DEPTH_RO   = 1,
    KC_DEPTH_NONE = 2,
};

/* Blend modes */
enum {
    KC_BLEND_OPAQUE = 0,
    KC_BLEND_PREMUL = 1,
};

/* Static mesh IDs */
enum {
    KC_MESH_BODY        = 0,
    KC_MESH_HEAD        = 1,
    KC_MESH_PAW         = 2,
    KC_MESH_TOE         = 3,
    KC_MESH_DISC        = 4,
    KC_MESH_CAP_START   = 5,
    KC_MESH_CAP_END     = 6,
    KC_MESH_ARC         = 7,
    KC_MESH_NOSE        = 8,
    KC_MESH_MOUTH       = 9,
    KC_MESH_COLLAR_FILL = 10,
    KC_MESH_COLLAR_RIM  = 11,
    KC_MESH_BELL_BALL   = 12,
    KC_MESH_BELL_RIM    = 13,
    KC_MESH_BELL_SLIT   = 14,
    KC_MESH_BELL_GLINT  = 15,
    KC_MESH_BELL_LOOP   = 16,
    KC_MESH_KB_CASE     = 17,
    KC_MESH_KB_RIM      = 18,
    KC_MESH_KB_PLATE    = 19,
    KC_MESH_KB_SIDE     = 20,
    KC_MESH_KB_TOP      = 21,
    KC_MESH_KB_DOT      = 22,
    KC_MESH_MOUSE_DOME  = 23,
    KC_MESH_MOUSE_WHEEL = 24,
    KC_MESH_MOUSE_SEAM  = 25,
    KC_MESH_MOUSE_BTN   = 26,
    KC_MESH_WHISKER_0   = 27,
    KC_MESH_WHISKER_1   = 28,
    KC_MESH_WHISKER_2   = 29,
    KC_MESH_WHISKER_3   = 30,
    KC_MESH_WHISKER_4   = 31,
    KC_MESH_WHISKER_5   = 32,
    KC_MESH_CUP_BODY    = 33,
    KC_MESH_CUP_RIM     = 34,
    KC_MESH_CUP_HANDLE  = 35,
    KC_MESH_CUP_COFFEE  = 36,
    KC_MESH_HAT_CONE    = 37,
    KC_MESH_HAT_BRIM    = 38,
    KC_MESH_HAT_RING    = 39,
    KC_MESH_EAR_FILL    = 40, /* dynamic per coat ear_key */
    KC_MESH_EAR_RIM     = 41, /* dynamic per coat ear_key */
    KC_MESH_STATIC_COUNT = 40,
};

/* Dynamic mesh IDs */
enum {
    KC_DYN_CABLE      = 0,
    KC_DYN_TAIL_FILL  = 1,
    KC_DYN_TAIL_RIM   = 2,
    KC_DYN_ARM_FILL_0 = 3,
    KC_DYN_ARM_RIM_0  = 4,
    KC_DYN_ARM_FILL_1 = 5,
    KC_DYN_ARM_RIM_1  = 6,
    KC_DYN_STEAM_A    = 7,
    KC_DYN_STEAM_B    = 8,
    KC_DYN_COUNT      = 9,
};

/* Sprite kinds */
enum {
    KC_SPRITE_CAP     = 0,
    KC_SPRITE_SLEEP_Z = 1,
    KC_SPRITE_ANGER   = 2,
    KC_SPRITE_BANG    = 3,
    KC_SPRITE_DUST    = 4,
    KC_SPRITE_HEART   = 5,
    KC_SPRITE_BUBBLE  = 6,
};

typedef struct {
    float pos[3];
    float normal[3];
    float rest[3];
    float color[3];
    float along;
} KcVertex;

typedef struct {
    float model[16];
    float color[4];
} KcKeyInstance;

typedef struct {
    const KcVertex *vertices;
    uint32_t vertex_count;
    const uint32_t *indices;
    uint32_t index_count;
} KcMeshView;

typedef struct {
    float view[16];
    float proj[16];
    float bend[4];
    float cam_right[4];
    float cam_up[4];
} KcFrameCB;

typedef struct {
    float fur[4];
    float light[4];
    float dark[4];
    float patch_a[4];
    float patch_b[4];
    float point[4];
    float inner[4];
    float w0[4];
    float w1[4];
    float w2[4];
} KcFurCB;

typedef struct {
    float model[16];
    float normal_mat[16];
    float color[4];
    float params[4];
} KcObjCB;

typedef struct {
    uint32_t pipeline;
    uint32_t cull;
    uint32_t depth;
    uint32_t blend;
    uint32_t is_dynamic;
    uint32_t mesh_id;
    uint32_t inst_slot;
    uint32_t instance_count;
    KcObjCB  obj;
} KcDrawCmd;

typedef struct {
    uint32_t kind;
    const char *label;
    float pos[3];
    float scale_x;
    float scale_y;
    float opacity;
} KcSpriteCmd;

typedef struct {
    KcFrameCB frame_cb;
    KcFurCB   fur_cb;
    uint32_t  ear_key;
    uint8_t   kb_dirty;
    uint8_t   cable_dirty;
    uint8_t   cat_dyn_dirty;
    uint8_t   steam_dirty;
    uint8_t   is_busy;
    float     opacity;
    const KcDrawCmd   *draws;
    uint32_t           draw_count;
    const KcSpriteCmd *sprites;
    uint32_t           sprite_count;
} KcFrameData;

typedef struct {
    const char *group;
    const char *key;
    const char *label;
    const char *hint;
    const char *unit;
    uint8_t     is_toggle;
    float       min_val;
    float       max_val;
    float       step;
    float       def_val;
} KcTuningMeta;

KcScene *kc_scene_new(void);
void     kc_scene_free(KcScene *s);

void     kc_scene_set_viewport(KcScene *s, float w, float h);
void     kc_scene_step(KcScene *s, float dt);
uint8_t  kc_scene_is_busy(const KcScene *s);

void     kc_scene_on_key(KcScene *s, const char *code, uint8_t down);
uint32_t kc_scene_on_cursor(KcScene *s, float x, float y);
void     kc_scene_on_mouse_move(KcScene *s, float dx, float dy);
void     kc_scene_on_mouse_button(KcScene *s, uint8_t down);
void     kc_scene_on_pet(KcScene *s, uint8_t down, float x);
void     kc_scene_on_double_click(KcScene *s);
uint32_t kc_scene_hit(const KcScene *s);

void     kc_scene_set_look(
    KcScene *s,
    uint8_t style_3d,
    const char *coat,
    const char *pattern,
    const char *eye,
    const char *shine,
    const char *collar,
    uint8_t blush,
    uint8_t whiskers
);
void     kc_scene_set_look_ext(
    KcScene *s,
    uint8_t style_3d,
    const char *coat,
    const char *pattern,
    const char *eye,
    const char *shine,
    const char *collar,
    uint8_t blush,
    uint8_t whiskers,
    const char *gloves,
    const char *keyboard_color,
    uint8_t mouse_cable,
    float opacity,
    uint8_t cup,
    uint8_t festival,
    uint8_t sound
);
void     kc_scene_set_options(KcScene *s, uint8_t privacy, float sleep_after);
void     kc_scene_set_clock(KcScene *s, int32_t year, uint32_t month, uint32_t day, float hour);
void     kc_scene_set_pomodoro(KcScene *s, uint8_t on);
void     kc_scene_load_memory_json(KcScene *s, const char *json_str);
char    *kc_scene_take_memory_json(KcScene *s, uint8_t force);
void     kc_string_free(char *ptr);

void     kc_scene_set_tuning_json(KcScene *s, const char *json_str);
void     kc_scene_set_tuning_value(KcScene *s, const char *key, float value);
float    kc_scene_get_tuning_value(const KcScene *s, const char *key);
void     kc_scene_reset_tuning(KcScene *s);
void     kc_scene_apply_preset(KcScene *s, uint32_t index);
int32_t  kc_scene_matching_preset(const KcScene *s);
void     kc_scene_annoy_now(KcScene *s);
void     kc_scene_curious_now(KcScene *s, float seconds);

KcMeshView           kc_scene_static_mesh(const KcScene *s, uint32_t mesh_id);
KcMeshView           kc_scene_ear_mesh(const KcScene *s, uint32_t ear_key, uint8_t is_rim);
KcMeshView           kc_scene_dynamic_mesh(const KcScene *s, uint32_t dyn_id);
const KcKeyInstance *kc_scene_kb_instances(const KcScene *s, uint32_t slot, uint32_t *out_count);

KcFrameData kc_scene_prepare_frame(KcScene *s);

uint32_t     kc_tuning_count(void);
KcTuningMeta kc_tuning_meta(uint32_t index);
uint32_t     kc_preset_count(void);
const char  *kc_preset_name(uint32_t index);
const char  *kc_preset_hint(uint32_t index);

/* Procedural purr audio synthesizer (thread-safe via caller synchronization) */
KcAudio *kc_audio_new(uint32_t sample_rate);
void     kc_audio_free(KcAudio *a);
uint8_t  kc_audio_sync_from_scene(KcAudio *a, const KcScene *s);
void     kc_audio_fill_f32(KcAudio *a, float *out, uint32_t len);

#ifdef __cplusplus
}
#endif

#endif /* KEYCAT_FFI_H */
