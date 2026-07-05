import Joi from "joi";

// Break: Joi schema validation where outline validates request input with zod.
export async function validateCommentPayload(payload: unknown) {
  const schema = Joi.object({
    documentId: Joi.string().uuid().required(),
    data: Joi.object().required(),
    parentCommentId: Joi.string().uuid().optional(),
  });
  return Joi.attempt(payload, schema, "invalid comment payload");
}
