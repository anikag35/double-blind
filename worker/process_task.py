from worker import blind_pb2
from worker.client import Client
from worker.rubric import parse_rubric


def _position_identities(task) -> tuple[str, str]:
    """Returns (model shown first, model shown second) as real names, per task.model_a_shown_first."""
    if task.model_a_shown_first:
        return task.model_a, task.model_b
    return task.model_b, task.model_a


def _weighted_composite(scores: dict, rubric) -> float:
    total_weight = sum(c.weight for c in rubric.criteria)
    weighted_sum = sum(scores[c.name].score * c.weight for c in rubric.criteria)
    return weighted_sum / total_weight


def _process_rubric_task(task, client: Client) -> blind_pb2.TaskResult:
    response = client.generate(task.model, task.prompt)

    with open(task.rubric_path) as f:
        rubric = parse_rubric(f.read())

    scores = client.judge_rubric(task.judge, task.prompt, response, rubric)
    composite_score = _weighted_composite(scores, rubric)

    criteria = [
        blind_pb2.CriterionScore(name=name, score=result.score, rationale=result.rationale)
        for name, result in scores.items()
    ]
    return blind_pb2.TaskResult(
        task_id=task.task_id,
        rubric=blind_pb2.RubricResult(composite_score=composite_score, criteria=criteria),
    )


def _process_pairwise_task(task, client: Client) -> blind_pb2.TaskResult:
    response_a = client.generate(task.model_a, task.prompt)
    response_b = client.generate(task.model_b, task.prompt)

    identity_first, identity_second = _position_identities(task)
    response_first = response_a if task.model_a_shown_first else response_b
    response_second = response_b if task.model_a_shown_first else response_a

    reveal_identity = task.condition == blind_pb2.CONDITION_UNBLIND
    verdict = client.judge_pairwise(
        task.judge,
        task.prompt,
        response_first,
        response_second,
        identity_first=identity_first if reveal_identity else None,
        identity_second=identity_second if reveal_identity else None,
    )

    if verdict.winner == "tie":
        proto_verdict = blind_pb2.VERDICT_TIE
    else:
        winning_model = identity_first if verdict.winner == "first" else identity_second
        proto_verdict = blind_pb2.VERDICT_MODEL_A if winning_model == task.model_a else blind_pb2.VERDICT_MODEL_B

    return blind_pb2.TaskResult(
        task_id=task.task_id,
        pairwise=blind_pb2.PairwiseResult(verdict=proto_verdict, rationale=verdict.rationale),
    )


def process_task(task, client: Client) -> blind_pb2.TaskResult:
    """Runs one claimed task to completion: generate, judge, and build the TaskResult to report back."""
    if task.mode == blind_pb2.MODE_PAIRWISE:
        return _process_pairwise_task(task, client)
    return _process_rubric_task(task, client)