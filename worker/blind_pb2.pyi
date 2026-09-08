from google.protobuf.internal import containers as _containers
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from collections.abc import Iterable as _Iterable, Mapping as _Mapping
from typing import ClassVar as _ClassVar, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class Mode(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MODE_UNSPECIFIED: _ClassVar[Mode]
    MODE_RUBRIC: _ClassVar[Mode]
    MODE_PAIRWISE: _ClassVar[Mode]

class Condition(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CONDITION_UNSPECIFIED: _ClassVar[Condition]
    CONDITION_BLIND: _ClassVar[Condition]
    CONDITION_UNBLIND: _ClassVar[Condition]

class Verdict(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    VERDICT_UNSPECIFIED: _ClassVar[Verdict]
    VERDICT_MODEL_A: _ClassVar[Verdict]
    VERDICT_MODEL_B: _ClassVar[Verdict]
    VERDICT_TIE: _ClassVar[Verdict]
MODE_UNSPECIFIED: Mode
MODE_RUBRIC: Mode
MODE_PAIRWISE: Mode
CONDITION_UNSPECIFIED: Condition
CONDITION_BLIND: Condition
CONDITION_UNBLIND: Condition
VERDICT_UNSPECIFIED: Verdict
VERDICT_MODEL_A: Verdict
VERDICT_MODEL_B: Verdict
VERDICT_TIE: Verdict

class RunRequest(_message.Message):
    __slots__ = ("models", "prompts_path", "judge", "rubric_path", "mode", "compare")
    MODELS_FIELD_NUMBER: _ClassVar[int]
    PROMPTS_PATH_FIELD_NUMBER: _ClassVar[int]
    JUDGE_FIELD_NUMBER: _ClassVar[int]
    RUBRIC_PATH_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    COMPARE_FIELD_NUMBER: _ClassVar[int]
    models: _containers.RepeatedScalarFieldContainer[str]
    prompts_path: str
    judge: str
    rubric_path: str
    mode: Mode
    compare: bool
    def __init__(self, models: _Optional[_Iterable[str]] = ..., prompts_path: _Optional[str] = ..., judge: _Optional[str] = ..., rubric_path: _Optional[str] = ..., mode: _Optional[_Union[Mode, str]] = ..., compare: _Optional[bool] = ...) -> None: ...

class RunHandle(_message.Message):
    __slots__ = ("run_id",)
    RUN_ID_FIELD_NUMBER: _ClassVar[int]
    run_id: str
    def __init__(self, run_id: _Optional[str] = ...) -> None: ...

class WorkerId(_message.Message):
    __slots__ = ("worker_id",)
    WORKER_ID_FIELD_NUMBER: _ClassVar[int]
    worker_id: str
    def __init__(self, worker_id: _Optional[str] = ...) -> None: ...

class Task(_message.Message):
    __slots__ = ("available", "task_id", "run_id", "mode", "judge", "rubric_path", "prompt", "prompt_hash", "model", "model_a", "model_b", "condition", "model_a_shown_first")
    AVAILABLE_FIELD_NUMBER: _ClassVar[int]
    TASK_ID_FIELD_NUMBER: _ClassVar[int]
    RUN_ID_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    JUDGE_FIELD_NUMBER: _ClassVar[int]
    RUBRIC_PATH_FIELD_NUMBER: _ClassVar[int]
    PROMPT_FIELD_NUMBER: _ClassVar[int]
    PROMPT_HASH_FIELD_NUMBER: _ClassVar[int]
    MODEL_FIELD_NUMBER: _ClassVar[int]
    MODEL_A_FIELD_NUMBER: _ClassVar[int]
    MODEL_B_FIELD_NUMBER: _ClassVar[int]
    CONDITION_FIELD_NUMBER: _ClassVar[int]
    MODEL_A_SHOWN_FIRST_FIELD_NUMBER: _ClassVar[int]
    available: bool
    task_id: str
    run_id: str
    mode: Mode
    judge: str
    rubric_path: str
    prompt: str
    prompt_hash: str
    model: str
    model_a: str
    model_b: str
    condition: Condition
    model_a_shown_first: bool
    def __init__(self, available: _Optional[bool] = ..., task_id: _Optional[str] = ..., run_id: _Optional[str] = ..., mode: _Optional[_Union[Mode, str]] = ..., judge: _Optional[str] = ..., rubric_path: _Optional[str] = ..., prompt: _Optional[str] = ..., prompt_hash: _Optional[str] = ..., model: _Optional[str] = ..., model_a: _Optional[str] = ..., model_b: _Optional[str] = ..., condition: _Optional[_Union[Condition, str]] = ..., model_a_shown_first: _Optional[bool] = ...) -> None: ...

class CriterionScore(_message.Message):
    __slots__ = ("name", "score", "rationale")
    NAME_FIELD_NUMBER: _ClassVar[int]
    SCORE_FIELD_NUMBER: _ClassVar[int]
    RATIONALE_FIELD_NUMBER: _ClassVar[int]
    name: str
    score: int
    rationale: str
    def __init__(self, name: _Optional[str] = ..., score: _Optional[int] = ..., rationale: _Optional[str] = ...) -> None: ...

class RubricResult(_message.Message):
    __slots__ = ("composite_score", "criteria")
    COMPOSITE_SCORE_FIELD_NUMBER: _ClassVar[int]
    CRITERIA_FIELD_NUMBER: _ClassVar[int]
    composite_score: float
    criteria: _containers.RepeatedCompositeFieldContainer[CriterionScore]
    def __init__(self, composite_score: _Optional[float] = ..., criteria: _Optional[_Iterable[_Union[CriterionScore, _Mapping]]] = ...) -> None: ...

class PairwiseResult(_message.Message):
    __slots__ = ("verdict", "rationale")
    VERDICT_FIELD_NUMBER: _ClassVar[int]
    RATIONALE_FIELD_NUMBER: _ClassVar[int]
    verdict: Verdict
    rationale: str
    def __init__(self, verdict: _Optional[_Union[Verdict, str]] = ..., rationale: _Optional[str] = ...) -> None: ...

class TaskResult(_message.Message):
    __slots__ = ("task_id", "rubric", "pairwise")
    TASK_ID_FIELD_NUMBER: _ClassVar[int]
    RUBRIC_FIELD_NUMBER: _ClassVar[int]
    PAIRWISE_FIELD_NUMBER: _ClassVar[int]
    task_id: str
    rubric: RubricResult
    pairwise: PairwiseResult
    def __init__(self, task_id: _Optional[str] = ..., rubric: _Optional[_Union[RubricResult, _Mapping]] = ..., pairwise: _Optional[_Union[PairwiseResult, _Mapping]] = ...) -> None: ...

class Empty(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class HeartbeatRequest(_message.Message):
    __slots__ = ("task_id", "worker_id")
    TASK_ID_FIELD_NUMBER: _ClassVar[int]
    WORKER_ID_FIELD_NUMBER: _ClassVar[int]
    task_id: str
    worker_id: str
    def __init__(self, task_id: _Optional[str] = ..., worker_id: _Optional[str] = ...) -> None: ...

class RunId(_message.Message):
    __slots__ = ("run_id",)
    RUN_ID_FIELD_NUMBER: _ClassVar[int]
    run_id: str
    def __init__(self, run_id: _Optional[str] = ...) -> None: ...

class LeaderboardEntry(_message.Message):
    __slots__ = ("rank", "model", "mean_score", "ci_low", "ci_high")
    RANK_FIELD_NUMBER: _ClassVar[int]
    MODEL_FIELD_NUMBER: _ClassVar[int]
    MEAN_SCORE_FIELD_NUMBER: _ClassVar[int]
    CI_LOW_FIELD_NUMBER: _ClassVar[int]
    CI_HIGH_FIELD_NUMBER: _ClassVar[int]
    rank: int
    model: str
    mean_score: float
    ci_low: float
    ci_high: float
    def __init__(self, rank: _Optional[int] = ..., model: _Optional[str] = ..., mean_score: _Optional[float] = ..., ci_low: _Optional[float] = ..., ci_high: _Optional[float] = ...) -> None: ...

class Leaderboard(_message.Message):
    __slots__ = ("run_id", "entries")
    RUN_ID_FIELD_NUMBER: _ClassVar[int]
    ENTRIES_FIELD_NUMBER: _ClassVar[int]
    run_id: str
    entries: _containers.RepeatedCompositeFieldContainer[LeaderboardEntry]
    def __init__(self, run_id: _Optional[str] = ..., entries: _Optional[_Iterable[_Union[LeaderboardEntry, _Mapping]]] = ...) -> None: ...
