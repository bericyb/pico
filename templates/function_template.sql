CREATE OR REPLACE FUNCTION {name}(example_parameter int)
RETURNS TABLE(example_result text) AS $$
	<SQL STATEMENTS>;
$$ LANGUAGE sql;