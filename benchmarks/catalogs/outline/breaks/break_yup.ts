import { object as yupObject, string as yupString } from "yup";

// Break: yup schema (aliased import) where outline validates request input with zod.
const accessRequestSchema = yupObject({
  documentId: yupString().uuid().required(),
  reason: yupString().max(500),
});

export async function assertAccessRequest(input: unknown) {
  return accessRequestSchema.validate(input, { abortEarly: false });
}
