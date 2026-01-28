#include "cupti.h"
#include "cupti_activity.h"
#include "cupti_checkpoint.h"
#include <cstdint>

using NV::Cupti::Checkpoint::CUpti_Checkpoint;

/// Re-define the macro for CUpti Checkpoint, as bindgen struggles to pick it up
static const size_t CUpti_Checkpoint_STRUCT_SIZE_VAR = CUpti_Checkpoint_STRUCT_SIZE;
