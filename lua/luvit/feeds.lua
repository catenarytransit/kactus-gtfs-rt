--prints a csv from github repo transitland-atlas/feeds with data "feed,vehicles_url,trips_url,alerts_url,isauth,isswiftly"
local json = require("json")

function loadJsonFile(filePath)
    local file = io.open(filePath, "r")
    local contents = ""
    local myTable = {}
    if file ~= nil then
        local contents = file:read( "*a" )
        myTable = json.decode(contents);
        io.close(file)
        return myTable
    end
end
--make sure to change this before running
local directoryPath = "/home/lolpro11/Documents/transitland-atlas/feeds/"
os.execute("bash -c 'cd "..directoryPath.."; git pull &> /dev/null'")
print("onestop,realtime_vehicle_positions,realtime_trip_updates,realtime_alerts,has_auth,auth_type,auth_header,auth_password,fetch_interval,multiauth")
local file_list = io.popen("ls -1 "..directoryPath.."/*.json")
local output = file_list:read("*all")
file_list:close()
local json_files = {}
for line in output:gmatch("[^\r\n]+") do
    table.insert(json_files, line)
end

for i=1, #json_files do
    local jsonData = loadJsonFile(json_files[i])
    for i=1, #jsonData.feeds do
        local output_str = ""
        local data_feed = jsonData.feeds[i]
        if data_feed.spec == "gtfs-rt" and data_feed.urls ~= nil then
            output_str = tostring(data_feed.id)
            for i=0, #data_feed. urls do
                if data_feed.urls.realtime_vehicle_positions ~= nil then
                    output_str = output_str..","..data_feed.urls.realtime_vehicle_positions
                else
                    output_str = output_str..","
                end
                if data_feed.urls.realtime_trip_updates ~= nil then
                    output_str = output_str..","..data_feed.urls.realtime_trip_updates
                else
                    output_str = output_str..","
                end
                if data_feed.urls.realtime_alerts ~= nil then
                    output_str = output_str..","..data_feed.urls.realtime_alerts
                else
                    output_str = output_str..","
                end
            end
            if data_feed.authorization ~= nil then
                --uncomment to keep vendors with auth
                goto done
                output_str = output_str..",true"
                if data_feed.authorization.type ~= nil then
                    output_str = output_str..","..data_feed.authorization.type
                end
                if data_feed.authorization.param_name ~= nil then
                    output_str = output_str..","..data_feed.authorization.param_name
                end
            else
                output_str = output_str..",false,,"
            end
            --auth_password
            output_str = output_str..","
            --fetch_interval
            output_str = output_str..",0.1"
            --multiauth
             output_str = output_str..","
        end
        if output_str ~= "" then
            print(output_str)
        end
        ::done::
    end
end
