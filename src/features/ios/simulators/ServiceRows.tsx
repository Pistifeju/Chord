import { useEffect, useState } from "react";
import type { ServiceRow } from "../../../ipc/bindings";
import { ChevronIcon } from "../../../ui/icons";
import { onOff, Segmented } from "../../../ui/Segmented";
import { Row } from "../../../ui/Row";
import { fail } from "../../../ui/alert";
import { groupSwitchIds, serviceDescription } from "./simulators";
import { loadServiceGroups, serviceGroups, setServiceKept } from "./store";

const keep = (ids: string[], kept: boolean) => setServiceKept(ids, kept).catch(fail);

function GroupRows({ group }: { group: ServiceRow }) {
  const [open, setOpen] = useState(false);
  const expandable = group.subgroups.length > 0;
  const label = (
    <>
      <span className="turn-chevron" data-open={open} data-hidden={!expandable || undefined}>
        <ChevronIcon />
      </span>
      <span>{group.name}</span>
      <span className="settings-description">{serviceDescription(group, group.subgroups)}</span>
    </>
  );
  const title = expandable ? (
    <button type="button" className="settings-disclosure" aria-expanded={open} onClick={() => setOpen(!open)}>
      {label}
    </button>
  ) : (
    <span className="settings-disclosure">{label}</span>
  );
  return (
    <>
      <Row title={title}>
        <Segmented<boolean> label={`Keep ${group.name} on`} options={onOff} value={group.kept ?? null} onChange={(kept) => keep(groupSwitchIds(group), kept)} />
      </Row>
      {open &&
        group.subgroups.map((subgroup) => (
          <Row key={subgroup.id} nested title={subgroup.name} description={<p className="settings-description">{serviceDescription(subgroup)}</p>}>
            <Segmented<boolean> label={`Keep ${subgroup.name} on`} options={onOff} value={subgroup.kept} onChange={(kept) => keep([subgroup.id], kept)} />
          </Row>
        ))}
    </>
  );
}

export function ServiceRows() {
  useEffect(() => {
    loadServiceGroups().catch(console.error);
  }, []);
  return serviceGroups.use().map((group) => <GroupRows key={group.id} group={group} />);
}
