--Joins any work object to its children
select other.metadata
from
    orm_resources self
    cross join jsonb_to_recordset(self.metadata->'member_ids') as  member_ids(id text)
    inner join orm_resources other on other.id::text = member_ids.id
where self.id::text = 'd93b14df-7c7f-4ec3-bd85-8c51246c409a';


--Joins any permission object to the resource to which it applies
select resource.*
from
    orm_resources permissions
    cross join jsonb_to_recordset(permissions.metadata->'access_to') as access_to(id text)
    inner join orm_resources resource on resource.id::text = access_to.id
where permissions.id::text = 'e22ad8b3-6029-47dc-9201-7ed8873ab5a8';
