// Break: react-hook-form useForm() form state instead of Dagit's controlled inputs + Recoil/useState.
// Dagit builds forms from @dagster-io/ui-components controls wired to local useState / Recoil. The
// react-hook-form useForm/register/handleSubmit API is a foreign form-state library not present in ui-core.
import * as React from 'react';
import {useForm} from 'react-hook-form';

interface SensorConfigForm {
  minIntervalSeconds: number;
  cursor: string;
}

export const SensorConfigEditor: React.FC<{onSave: (values: SensorConfigForm) => void}> = ({onSave}) => {
  const {register, handleSubmit, formState} = useForm<SensorConfigForm>({
    defaultValues: {minIntervalSeconds: 30, cursor: ''},
  });
  return (
    <form onSubmit={handleSubmit(onSave)}>
      <input type="number" {...register('minIntervalSeconds', {min: 1})} />
      <input {...register('cursor')} />
      <button type="submit" disabled={formState.isSubmitting}>
        Save
      </button>
    </form>
  );
};
