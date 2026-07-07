# ID: fastapi/encoders.py:98
def group_types_by_encoder(type_encoder_map):
    encoder_to_types = defaultdict(tuple)
    for target_type, encoder in type_encoder_map.items():
        encoder_to_types[encoder] += (target_type,)
    return encoder_to_types
